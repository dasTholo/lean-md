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
    pub outputs: Vec<(String, String)>, // (directive name, output) for capture=auto
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
fn parse_on_complete(rest: &str) -> Option<OnComplete> {
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
fn fire_action(ctx: &Rc<EngineContext>, _scope: &PhaseScope, action: &OnComplete) {
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
    if !body.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("@phase") || is_on_complete(t)
    }) {
        return render_markdown(ctx, body); // fast path, no phases or @on directives
    }

    let mut out = String::new();
    let mut buf = String::new(); // accumulating non-phase segment
    let mut scope: Option<PhaseScope> = None;

    for (idx, line) in body.lines().enumerate() {
        let src_line = idx + 1; // 1-based; @phase line = 1, first body line = 2
        let trimmed = line.trim_start();

        // @phase-end (close)
        if trimmed.starts_with("@phase-end") {
            match scope.take() {
                Some(sc) => finalize_phase(ctx, &mut out, &sc),
                None => out.push_str("<!-- lmd: stray @phase-end -->\n"),
            }
            continue;
        }

        // @phase "name" (open)
        if let Some(rest) = trimmed.strip_prefix("@phase") {
            // (rest may start with a space; `@phase-end` already handled above)
            if scope.is_some() {
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
            scope = Some(PhaseScope::new(name));
            continue;
        }

        // @on complete (only valid inside an open phase — D-8).
        if is_on_complete(trimmed) {
            match scope.as_mut() {
                None => out.push_str("<!-- lmd: @on complete outside @phase -->\n"),
                Some(sc) if sc.aborted.is_none() => {
                    let rest = trimmed.strip_prefix("@on").unwrap().trim_start();
                    let rest = rest.strip_prefix("complete").unwrap_or(rest).trim();
                    if let Some(action) = parse_on_complete(rest) {
                        sc.actions.push(action);
                    } else {
                        out.push_str("<!-- lmd: malformed @on complete -->\n");
                    }
                }
                Some(_) => {} // aborted: ignore further @on complete
            }
            continue;
        }

        // ordinary line: phase body or non-phase region
        if let Some(sc) = scope.as_mut() {
            if sc.aborted.is_some() {
                continue; // skip remaining body after abort
            }
            if let Some((name, args)) =
                crate::lmd::parser::block::parse_directive_line(line.as_bytes())
            {
                match crate::lmd::render::dispatch_result(ctx, &name, &args) {
                    Ok(rendered) => {
                        sc.outputs.push((name, rendered.clone()));
                        out.push_str(&rendered);
                        out.push('\n');
                    }
                    Err(e) => {
                        sc.aborted = Some(PhaseError {
                            phase: sc.name.clone(),
                            directive: name,
                            line: src_line,
                            cause: format!("{e:?}"),
                        });
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
    if let Some(sc) = scope.take() {
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
}
