//! `@phase`/`@phase-end` + `@on complete` executor (spec §3, §4). A pre-pass over
//! the body — structurally parallel to `macros::prune_containers` (line-scanned
//! containers), because phases are stateful/structural exactly like `@if`/`@define`
//! (which are also pre-passes, not registry bridges). Phase-free input takes a
//! zero-overhead fast path. Abort semantics + `@on complete` sinks land in later
//! tasks; Task 4 establishes scope + structural validation + the open sink.

use std::rc::Rc;

use crate::lmd::engine::{EngineContext, render_markdown};

/// A failing body directive that aborted its phase (spec §3.3, D-9). Self-
/// describing: which directive failed, where, why. The single source for all
/// three abort sinks (envelope + decision + gotcha). Populated in Task 5.
#[derive(Debug, Clone)]
pub struct PhaseError {
    pub phase: String,
    pub directive: String,
    pub line: usize,
    pub cause: String,
}

impl PhaseError {
    /// Byte-stable render envelope (spec §3.3, #498 — no timestamp/counter).
    pub fn envelope(&self) -> String {
        format!(
            "PHASE_ABORTED \"{}\" at @{} (line {}): {}",
            self.phase, self.directive, self.line, self.cause
        )
    }
    /// Session-decision narrative derived from the same source.
    pub fn decision_summary(&self) -> String {
        format!(
            "Phase aborted: {} — @{} (line {}) failed: {}",
            self.phase, self.directive, self.line, self.cause
        )
    }
}

/// One accumulated `@on complete` action (populated in Task 7).
#[derive(Debug, Clone)]
pub(crate) struct OnComplete {
    pub sink: String,
    pub value: String,
    pub attrs: Vec<(String, String)>,
}

/// Per-phase render scope (one stack entry; v1 phases are flat, no nesting).
pub(crate) struct PhaseScope {
    pub name: String,
    pub actions: Vec<OnComplete>,
    /// `(directive name, raw args string, output)` — args preserved so
    /// `capture=auto` can reconstruct the search pattern for `auto_findings`.
    pub outputs: Vec<(String, String, String)>,
    pub aborted: Option<PhaseError>,
}

impl PhaseScope {
    fn new(name: String) -> Self {
        Self {
            name,
            actions: Vec::new(),
            outputs: Vec::new(),
            aborted: None,
        }
    }
}

/// Parse `<sink>="<value>" [attr=v …]` into an [`OnComplete`]. Reuses
/// [`crate::lmd::args::DirectiveArgs`] for quote-aware tokenization. The first
/// named pair's key is the sink; its value is the payload; remaining pairs are
/// attrs (e.g. `category=`, `key=`, `confidence=`).
///
/// Valueless sinks (`capture=auto`, `checkpoint`) are detected first — they
/// carry their payload inline as `key=value` tokens rather than quoted strings,
/// so the generic pair-parser would split them incorrectly.
fn parse_on_complete(rest: &str) -> Option<OnComplete> {
    let head = rest.split_whitespace().next().unwrap_or("");
    if head == "capture=auto" {
        return Some(OnComplete {
            sink: "capture".into(),
            value: "auto".into(),
            attrs: vec![],
        });
    }
    if head == "checkpoint" {
        return Some(OnComplete {
            sink: "checkpoint".into(),
            value: String::new(),
            attrs: vec![],
        });
    }
    let args = crate::lmd::args::DirectiveArgs::parse(rest);
    let pairs = args.named_pairs();
    let (sink, value) = pairs.first()?.clone();
    let attrs = pairs[1..].to_vec();
    Some(OnComplete { sink, value, attrs })
}

/// Look up an attribute value by key from a `Vec<(String, String)>` attrs list.
fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Fire one accumulated action against the appropriate sink. Session sinks
/// (task/finding/decision) handled here; knowledge sink (Task 8) wired.
/// Team (Task 9), capture/checkpoint (Task 10) extend this match in later tasks.
fn fire_action(ctx: &Rc<EngineContext>, scope: &PhaseScope, action: &OnComplete) {
    let value = if action.value.contains("{{") {
        crate::lmd::macros::eval_string(ctx, &action.value)
    } else {
        action.value.clone()
    };
    match action.sink.as_str() {
        "task" => session_set_task(ctx, &value),
        "finding" => session_add_finding(ctx, &value),
        "decision" => session_decision(ctx, &value),
        "remember" => {
            let category = attr(&action.attrs, "category").unwrap_or("decision");
            let key = attr(&action.attrs, "key").map_or_else(
                || crate::lmd::bridges::remember::slug(&value),
                str::to_string,
            );
            let confidence = attr(&action.attrs, "confidence")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.8);
            let _ = crate::lmd::bridges::remember::knowledge_remember(
                ctx, category, &key, &value, confidence,
            );
        }
        "post" => fire_agent(ctx, "post", &value, attr(&action.attrs, "category")),
        "diary" => fire_agent(ctx, "diary", &value, attr(&action.attrs, "category")),
        "capture" => {
            // value == "auto": scan body tool outputs → session findings.
            // Determinism #498: no output written to the render body.
            let Some(sinks) = ctx.sinks.as_ref() else {
                return;
            };
            let Some(session) = sinks.session.as_ref() else {
                return;
            };
            for (name, args, output) in &scope.outputs {
                let tool = format!("ctx_{name}");
                // ctx_search output has no `pattern:` header (the tool omits it in its text
                // output); inject a synthetic header from the directive's raw args string so
                // `extract_ctx_search` can identify the pattern and skip the low-signal guard.
                let annotated;
                let effective = if name == "search" {
                    let pattern = args.split_whitespace().next().unwrap_or(name.as_str());
                    annotated = format!("pattern: \"{pattern}\"\n{output}");
                    &annotated
                } else {
                    output
                };
                if let Some(f) = crate::core::auto_findings::extract(&tool, effective) {
                    session.blocking_write().add_finding(None, None, &f.summary);
                }
            }
        }
        "checkpoint" => {
            // Compress over the session cache as a side-effect/log (#498: output discarded).
            if ctx.sinks.is_none() {
                return;
            }
            let _ = crate::tools::ctx_compress::handle(
                &ctx.cache.borrow(),
                false,
                crate::core::protocol::CrpMode::Off,
            );
        }
        _ => {} // other sinks land in later tasks
    }
}

/// Team-bus / diary sink. Degrades to no-op when no agent is registered (§4.4).
fn fire_agent(ctx: &Rc<EngineContext>, action: &str, message: &str, category: Option<&str>) {
    let Some(sinks) = ctx.sinks.as_ref() else {
        return;
    };
    let Some(agent_id) = sinks.agent_id.as_deref() else {
        return;
    }; // no agent → no-op
    let root = ctx.jail_root.to_str().unwrap_or(".");
    let _ = crate::tools::ctx_agent::handle(
        action,
        None,
        None,
        root,
        Some(agent_id),
        Some(message),
        category,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
    );
}

fn session_set_task(ctx: &Rc<EngineContext>, value: &str) {
    let Some(sinks) = ctx.sinks.as_ref() else {
        return;
    };
    let Some(session) = sinks.session.as_ref() else {
        return;
    };
    session.blocking_write().set_task(value, None);
}

fn session_add_finding(ctx: &Rc<EngineContext>, value: &str) {
    let Some(sinks) = ctx.sinks.as_ref() else {
        return;
    };
    let Some(session) = sinks.session.as_ref() else {
        return;
    };
    session.blocking_write().add_finding(None, None, value);
}

/// Session `decision` sink (open + abort narrative). Gated by sinks; headless
/// no-op. Uses `blocking_write()` — the MCP handler runs render under spawn_blocking.
pub(crate) fn session_decision(ctx: &Rc<EngineContext>, summary: &str) {
    let Some(sinks) = ctx.sinks.as_ref() else {
        return;
    };
    let Some(session) = sinks.session.as_ref() else {
        return;
    };
    session.blocking_write().add_decision(summary, None);
}

/// Pass 4: execute phase blocks. Phase-free body → fast pass-through.
pub fn render_with_phases(ctx: &Rc<EngineContext>, body: &str) -> String {
    // Fast path: no `@phase` or `@on complete` in this body → plain render.
    // Even when an outer phase is open on ctx.phase_scope (nested `@call`
    // expansion), a body with no phase directives contributes nothing to the
    // outer scope — skip the line scanner entirely.
    let has_phase_directives = body.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("@phase") || is_on_complete(t)
    });
    if !has_phase_directives {
        return render_markdown(ctx, body); // fast path, nothing to do
    }

    // Nested call (e.g. from @call body): outer phase is open on the stack.
    // This body may contain `@on complete` lines that belong to the outer
    // phase — the line scanner collects them without opening a new scope.
    let outer_phase_open = !ctx.phase_scope.borrow().is_empty();
    let nested_passthrough = outer_phase_open;

    let mut out = String::new();
    let mut buf = String::new(); // accumulating non-phase segment
    // Track whether THIS call opened a scope entry (so we know to pop on our
    // @phase-end, not a stray one from an outer caller).
    let mut opened_here = false;

    for (idx, line) in body.lines().enumerate() {
        let src_line = idx + 1; // 1-based; @phase line = 1, first body line = 2
        let trimmed = line.trim_start();

        // @phase-end (close) — only meaningful when we opened a scope here.
        if trimmed.starts_with("@phase-end") {
            if opened_here {
                let sc = ctx.phase_scope.borrow_mut().pop();
                match sc {
                    Some(sc) => finalize_phase(ctx, &mut out, &sc),
                    None => out.push_str("<!-- lmd: stray @phase-end -->\n"),
                }
                opened_here = false;
            } else if !nested_passthrough {
                out.push_str("<!-- lmd: stray @phase-end -->\n");
            }
            // In nested_passthrough mode, @phase-end belongs to the outer
            // caller — don't consume it here; but since we're line-scanning
            // the already-expanded body, it won't appear here anyway.
            continue;
        }

        // @phase "name" (open)
        if let Some(rest) = trimmed.strip_prefix("@phase") {
            // (rest may start with a space; `@phase-end` already handled above)
            if opened_here || outer_phase_open {
                out.push_str("<!-- lmd: nested @phase -->\n");
                continue;
            }
            // flush pending non-phase markdown before opening
            if !buf.is_empty() {
                out.push_str(&render_markdown(ctx, &buf));
                buf.clear();
            }
            let name = parse_phase_name(rest);
            session_decision(ctx, &format!("Phase: {name}"));
            ctx.phase_scope.borrow_mut().push(PhaseScope::new(name));
            opened_here = true;
            continue;
        }

        // @on complete (only valid inside an open phase — D-8).
        if is_on_complete(trimmed) {
            let has_open = !ctx.phase_scope.borrow().is_empty();
            if has_open {
                let aborted = ctx
                    .phase_scope
                    .borrow()
                    .last()
                    .is_some_and(|sc| sc.aborted.is_some());
                if !aborted {
                    let rest = trimmed.strip_prefix("@on").unwrap().trim_start();
                    let rest = rest.strip_prefix("complete").unwrap_or(rest).trim();
                    if let Some(action) = parse_on_complete(rest) {
                        if let Some(sc) = ctx.phase_scope.borrow_mut().last_mut() {
                            sc.actions.push(action);
                        }
                    } else {
                        out.push_str("<!-- lmd: malformed @on complete -->\n");
                    }
                }
                // aborted: ignore further @on complete
            } else {
                out.push_str("<!-- lmd: @on complete outside @phase -->\n");
            }
            continue;
        }

        // ordinary line: phase body or non-phase region
        let in_phase = !ctx.phase_scope.borrow().is_empty();
        if in_phase {
            let aborted = ctx
                .phase_scope
                .borrow()
                .last()
                .is_some_and(|sc| sc.aborted.is_some());
            if aborted {
                continue; // skip remaining body after abort
            }
            if let Some((name, args)) =
                crate::lmd::parser::block::parse_directive_line(line.as_bytes())
            {
                match crate::lmd::render::dispatch_result(ctx, &name, &args) {
                    Ok(rendered) => {
                        if let Some(sc) = ctx.phase_scope.borrow_mut().last_mut() {
                            sc.outputs.push((name, args.clone(), rendered.clone()));
                        }
                        out.push_str(&rendered);
                        out.push('\n');
                    }
                    Err(e) => {
                        let phase_name = ctx
                            .phase_scope
                            .borrow()
                            .last()
                            .map_or_else(String::new, |sc| sc.name.clone());
                        if let Some(sc) = ctx.phase_scope.borrow_mut().last_mut() {
                            sc.aborted = Some(PhaseError {
                                phase: phase_name,
                                directive: name,
                                line: src_line,
                                cause: format!("{e:?}"),
                            });
                        }
                    }
                }
            } else {
                out.push_str(&render_markdown(ctx, line));
                out.push('\n');
            }
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }

    // trailing flushes
    if opened_here {
        // Phase was opened here but not closed — unterminated.
        let sc = ctx.phase_scope.borrow_mut().pop();
        out.push_str("<!-- lmd: unterminated @phase -->\n");
        let _ = sc;
    }
    if !buf.is_empty() {
        out.push_str(&render_markdown(ctx, &buf));
    }
    out
}

/// Close a phase: if aborted emit the `PHASE_ABORTED` envelope + session
/// decision and skip accumulated `@on complete` (spec §3.2 step 2).
/// Clean close fires accumulated `@on complete` (Task 7).
fn finalize_phase(ctx: &Rc<EngineContext>, out: &mut String, scope: &PhaseScope) {
    if let Some(err) = scope.aborted.as_ref() {
        out.push_str(&err.envelope());
        out.push('\n');
        session_decision(ctx, &err.decision_summary());
        report_phase_gotcha(ctx, err); // third abort sink (spec §3.5, D-10)
        return; // skip accumulated @on complete (spec §3.2 step 2)
    }
    // clean close: fire accumulated @on complete in source order
    for action in &scope.actions {
        fire_action(ctx, scope, action);
    }
}

/// Third abort sink (spec §3.5, D-10): feed the PhaseError into the bug-memory
/// store. Load-by-root, gated by sinks. `report_gotcha` sets source=AgentReported
/// (0.9) — justified: the engine is an authoritative reporter, not a heuristic.
fn report_phase_gotcha(ctx: &Rc<EngineContext>, err: &PhaseError) {
    let Some(sinks) = ctx.sinks.as_ref() else {
        return;
    };
    let root = ctx.jail_root.to_str().unwrap_or(".");
    let mut store = crate::core::gotcha_tracker::GotchaStore::load(root);
    let trigger = normalize_trigger(err);
    let resolution = format!(
        "resolve @{} failure in phase {}: {}",
        err.directive, err.phase, err.cause
    );
    store.report_gotcha(
        &trigger,
        &resolution,
        map_cause_category(&err.cause),
        "warning",
        &sinks.session_id,
    );
    let _ = store.save(root);
}

/// Merge-stable, greppable trigger: paths stripped, cause reduced to its head.
fn normalize_trigger(err: &PhaseError) -> String {
    let head = err
        .cause
        .split([':', '('])
        .next()
        .unwrap_or(&err.cause)
        .trim();
    format!(
        "@phase \"{}\" aborted at @{}: {}",
        err.phase, err.directive, head
    )
}

/// Deterministic cause → GotchaCategory loose-name mapping (spec §3.5).
fn map_cause_category(cause: &str) -> &'static str {
    if cause.contains("error[E") || cause.contains("mismatched") {
        "build"
    } else if cause.starts_with("Io")
        || cause.starts_with("MissingArg")
        || cause.starts_with("Resolve")
        || cause.contains("FILE_NOT_FOUND")
        || cause.contains("BACKEND_REQUIRED")
    {
        "config"
    } else {
        "other"
    }
}

/// `@phase "Parser"` / `@phase Parser` → `Parser` (quotes optional, trimmed).
fn parse_phase_name(rest: &str) -> String {
    rest.trim().trim_matches('"').trim().to_string()
}

/// True for an `@on complete …` line (name `on`, first arg token `complete`).
fn is_on_complete(trimmed: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix("@on") {
        return rest.trim_start().starts_with("complete");
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::lmd::engine::render;

    #[test]
    fn open_phase_renders_body() {
        let out = render("@phase \"P\"\nhello\n@phase-end\n");
        assert!(out.contains("hello"), "phase body must render: {out}");
        assert!(
            !out.contains("@phase"),
            "phase markers must not leak: {out}"
        );
    }

    #[test]
    fn unterminated_phase_is_a_visible_error() {
        let out = render("@phase \"P\"\nhello\n");
        assert!(out.contains("unterminated @phase"), "got: {out}");
    }

    #[test]
    fn nested_phase_is_an_error() {
        let out = render("@phase \"A\"\n@phase \"B\"\n@phase-end\n@phase-end\n");
        assert!(out.contains("nested @phase"), "got: {out}");
    }

    #[test]
    fn on_complete_outside_phase_is_an_error() {
        let out = render("@on complete task=\"x\"\n");
        assert!(out.contains("@on complete outside @phase"), "got: {out}");
    }

    #[test]
    fn stray_phase_end_is_an_error() {
        let out = render("@phase-end\n");
        assert!(out.contains("stray @phase-end"), "got: {out}");
    }

    #[test]
    fn phase_free_body_is_unaffected() {
        let out = render("plain text\n");
        assert!(out.contains("plain text"));
    }

    #[test]
    fn body_error_aborts_phase_with_stable_envelope() {
        // @read of a missing file errors → phase aborts, PHASE_ABORTED envelope.
        let out =
            render("@phase \"Parser\"\n@read /no/such/file_xyz.rs\nAFTER\n@phase-end\nNEXT\n");
        assert!(
            out.contains("PHASE_ABORTED \"Parser\" at @read"),
            "envelope missing: {out}"
        );
        assert!(
            out.contains("(line 2)"),
            "1-based source line of failing directive: {out}"
        );
        assert!(
            !out.contains("AFTER"),
            "post-error body must be skipped: {out}"
        );
        assert!(
            out.contains("NEXT"),
            "render continues after the phase: {out}"
        );
    }

    #[test]
    fn clean_phase_dispatches_body_directives() {
        // A successful directive in the body renders its output (no abort).
        let dir = std::env::temp_dir().join("lmd_phase_clean");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.cnt"), "x").unwrap();
        let pat = format!("{}/*.cnt", dir.to_str().unwrap());
        let out = render(&format!("@phase \"Count\"\n@count {pat}\n@phase-end\n"));
        assert!(
            out.contains('1'),
            "body directive output must render: {out}"
        );
        assert!(
            !out.contains("PHASE_ABORTED"),
            "clean phase must not abort: {out}"
        );
    }

    #[test]
    fn abort_reports_gotcha_with_normalized_trigger() {
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;

        let root = std::env::temp_dir().join("lmd_phase_gotcha");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let sinks = SinkHandles {
            session_id: "s-gotcha".to_string(),
            session: None,
            agent_id: None,
        };
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            root.clone(),
            sinks,
        ));
        let out = crate::lmd::engine::render_body(
            &ctx,
            "@phase \"Parser\"\n@read /no/such/file_abc.rs\n@phase-end\n",
        );
        assert!(
            out.contains("PHASE_ABORTED"),
            "envelope still emitted: {out}"
        );

        // The gotcha was persisted load-by-root and merges on repeat.
        let store = crate::core::gotcha_tracker::GotchaStore::load(root.to_str().unwrap());
        let listing = store.format_list();
        assert!(
            listing.contains("Parser"),
            "gotcha trigger must name the phase: {listing}"
        );
        assert!(
            !listing.contains("/no/such/"),
            "paths must be stripped from trigger: {listing}"
        );
    }

    #[test]
    fn cause_category_mapping_is_deterministic() {
        use super::map_cause_category;
        assert_eq!(map_cause_category("Io(\"no file\")"), "config");
        assert_eq!(map_cause_category("MissingArg(\"path\")"), "config");
        assert_eq!(map_cause_category("error[E0433] mismatched"), "build");
        assert_eq!(map_cause_category("something else"), "other");
    }

    #[test]
    fn on_complete_fires_session_sinks_in_order_on_clean_end() {
        use crate::core::session::SessionState;
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let session = Arc::new(RwLock::new(SessionState::new()));
        let sinks = SinkHandles {
            session_id: "s1".to_string(),
            session: Some(session.clone()),
            agent_id: None,
        };
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            std::env::temp_dir(),
            sinks,
        ));
        let _ = crate::lmd::engine::render_body(
            &ctx,
            "@phase \"Build\"\nbody\n@on complete task=\"build done [100%]\"\n@on complete decision=\"shipped\"\n@phase-end\n",
        );
        let st = session.blocking_read();
        assert_eq!(
            st.task.as_ref().unwrap().description,
            "build done [100%]",
            "task sink must fire"
        );
        assert!(
            st.decisions.iter().any(|d| d.summary.contains("shipped")),
            "decision sink must fire"
        );
    }

    #[test]
    fn on_complete_remember_writes_knowledge() {
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;

        let root = std::env::temp_dir().join("lmd_oc_remember");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            root.clone(),
            SinkHandles {
                session_id: "s3".into(),
                session: None,
                agent_id: None,
            },
        ));
        let _ = crate::lmd::engine::render_body(
            &ctx,
            "@phase \"P\"\nwork\n@on complete remember=\"parser uses pratt\" category=decision\n@phase-end\n",
        );
        let k = crate::core::knowledge::ProjectKnowledge::load(root.to_str().unwrap()).unwrap();
        let hits = k.recall("pratt");
        assert!(
            !hits.is_empty(),
            "remember sink must persist a knowledge fact"
        );
        // Verify attrs were persisted correctly (guards against attr-lookup bugs).
        let fact = hits
            .iter()
            .find(|f| f.key == "parser_uses_pratt")
            .expect("fact key must be slug of content: 'parser_uses_pratt'");
        assert_eq!(
            fact.category, "decision",
            "category attr must be 'decision'"
        );
        assert!(
            fact.value.contains("parser uses pratt"),
            "value must contain the written content; got: {:?}",
            fact.value
        );
    }

    #[test]
    fn post_without_agent_degrades_to_noop() {
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;

        // sinks present but no agent_id → post/diary degrade, no panic, no error envelope.
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            std::env::temp_dir(),
            SinkHandles {
                session_id: "s4".into(),
                session: None,
                agent_id: None,
            },
        ));
        let out = crate::lmd::engine::render_body(
            &ctx,
            "@phase \"P\"\nwork\n@on complete post=\"hi\" category=status\n@phase-end\n",
        );
        assert!(
            !out.contains("PHASE_ABORTED"),
            "team sink degradation is not an abort: {out}"
        );
        assert!(!out.contains("panic"), "must not panic: {out}");
    }

    #[test]
    fn aborted_phase_skips_on_complete() {
        use crate::core::session::SessionState;
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let session = Arc::new(RwLock::new(SessionState::new()));
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            std::env::temp_dir(),
            SinkHandles {
                session_id: "s2".into(),
                session: Some(session.clone()),
                agent_id: None,
            },
        ));
        let _ = crate::lmd::engine::render_body(
            &ctx,
            "@phase \"P\"\n@read /no/such/zzz.rs\n@on complete task=\"done [100%]\"\n@phase-end\n",
        );
        assert!(
            session.blocking_read().task.is_none(),
            "aborted phase must NOT fire its @on complete task"
        );
    }

    #[test]
    fn capture_auto_emits_findings_from_body_outputs() {
        use crate::core::session::SessionState;
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // A @search in the body produces output that auto_findings::extract turns
        // into a finding; capture=auto routes it to the session.
        let dir = std::env::temp_dir().join("lmd_capture_auto");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hit.rs"), "fn capture_marker_77() {}\n").unwrap();

        let session = Arc::new(RwLock::new(SessionState::new()));
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            dir.clone(),
            SinkHandles {
                session_id: "s5".into(),
                session: Some(session.clone()),
                agent_id: None,
            },
        ));
        let pat = "capture_marker_77";
        let _ = crate::lmd::engine::render_body(
            &ctx,
            &format!(
                "@phase \"Scan\"\n@search {pat} {}\n@on complete capture=auto\n@phase-end\n",
                dir.to_str().unwrap()
            ),
        );
        assert!(
            !session.blocking_read().findings.is_empty(),
            "capture=auto must turn body tool output into session findings"
        );
    }

    #[test]
    fn on_complete_substitutes_call_params() {
        use crate::core::session::SessionState;
        use crate::lmd::engine::{EngineContext, SinkHandles};
        use crate::lmd::header::LeanMdHeader;
        use std::rc::Rc;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let session = Arc::new(RwLock::new(SessionState::new()));
        let ctx = Rc::new(EngineContext::with_sinks(
            LeanMdHeader::default(),
            std::env::temp_dir(),
            SinkHandles {
                session_id: "s6".into(),
                session: Some(session.clone()),
                agent_id: None,
            },
        ));
        // A macro that closes a phase with parameterized task progress.
        let doc = "@define close(pct, note)\n\
                   @on complete task=\"{{ note }} [{{ pct }}%]\"\n\
                   @define-end\n\n\
                   @phase \"Parser\"\n\
                   work\n\
                   @call close(100, parser fertig) /\n\
                   @phase-end\n";
        let _ = crate::lmd::engine::render_body(&ctx, doc);
        let st = session.blocking_read();
        assert_eq!(
            st.task.as_ref().unwrap().description,
            "parser fertig [100%]",
            "@on complete inside @call must substitute params (D-5 composition)"
        );
    }

    #[test]
    fn phase_aborted_envelope_is_byte_stable() {
        use crate::lmd::engine::render;
        let doc = "@phase \"X\"\n@read /no/such/qq.rs\n@phase-end\n";
        let a = render(doc);
        let b = render(doc);
        assert_eq!(
            a, b,
            "render output must be a deterministic function of input (#498)"
        );
        assert!(a.contains("PHASE_ABORTED \"X\" at @read"));
    }
}
