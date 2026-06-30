//! `@phase`/`@phase-end` + `@on complete` executor (spec §3, §4). A pre-pass over
//! the body — structurally parallel to `macros::prune_containers` (line-scanned
//! containers), because phases are stateful/structural exactly like `@if`/`@define`
//! (which are also pre-passes, not registry bridges). Phase-free input takes a
//! zero-overhead fast path. Abort semantics + `@on complete` sinks land in later
//! tasks; Task 4 establishes scope + structural validation + the open sink.

use std::rc::Rc;

use crate::engine::{EngineContext, render_markdown};

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
/// [`crate::args::DirectiveArgs`] for quote-aware tokenization. The first
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
    let args = crate::args::DirectiveArgs::parse(rest);
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
        crate::macros::eval_string(ctx, &action.value)
    } else {
        action.value.clone()
    };
    match action.sink.as_str() {
        "task" => session_set_task(ctx, &value),
        "finding" => session_add_finding(ctx, &value),
        "decision" => session_decision(ctx, &value),
        "remember" => {
            let category = attr(&action.attrs, "category");
            let key = attr(&action.attrs, "key")
                .map_or_else(|| crate::bridges::remember::slug(&value), str::to_string);
            let confidence = attr(&action.attrs, "confidence").and_then(|s| s.parse::<f32>().ok());
            let _ = crate::bridges::remember::knowledge_remember(
                ctx, category, &key, &value, confidence,
            );
        }
        "post" => fire_agent(ctx, "post", &value, attr(&action.attrs, "category")),
        "diary" => fire_agent(ctx, "diary", &value, attr(&action.attrs, "category")),
        "capture" => {
            // value == "auto": scan body tool outputs → session findings via the
            // outbound `ctx_session` finding sink. Determinism #498: no output
            // written to the render body. Headless backend → BACKEND_REQUIRED
            // envelope is discarded (capture is a best-effort side-effect).
            for (name, args, output) in &scope.outputs {
                let tool = format!("ctx_{name}");
                // ctx_search output has no `pattern:` header (the tool omits it in its
                // text output); inject a synthetic header so `extract_search_pattern`
                // (in `auto_findings.rs`) finds a non-empty pattern and the
                // noise-guard doesn't fall back to "?" and discard the finding.
                // Parse via DirectiveArgs — identical to SearchBridge — so quoted
                // patterns (`@search "foo bar"`) and attr form (`@search pattern=foo`)
                // produce the correct label, not a malformed `"\"foo` or `pattern=foo`.
                let annotated;
                let effective = if name == "search" {
                    let parsed = crate::args::DirectiveArgs::parse(args);
                    let pattern = parsed
                        .positional(0)
                        .or_else(|| parsed.get("pattern"))
                        .unwrap_or(name.as_str());
                    annotated = format!("pattern: \"{pattern}\"\n{output}");
                    &annotated
                } else {
                    output
                };
                if let Some(f) = crate::auto_findings::extract(&tool, effective) {
                    session_add_finding(ctx, &f.summary);
                }
            }
        }
        "checkpoint" => {
            // Outbound checkpoint: ask the backend to compress the live session.
            // Headless → BACKEND_REQUIRED envelope, discarded (#498: no body output).
            let _ = ctx.backend.call(
                "ctx_compress",
                serde_json::json!({ "action": "checkpoint" }),
            );
        }
        _ => {} // other sinks land in later tasks
    }
}

/// Team-bus / diary sink → outbound `ctx_agent`. Degrades to a discarded
/// BACKEND_REQUIRED envelope when no backend/agent is reachable (§4.4); the
/// server enforces agent registration. Determinism #498: output not rendered.
fn fire_agent(ctx: &Rc<EngineContext>, action: &str, message: &str, category: Option<&str>) {
    let mut payload = serde_json::Map::new();
    payload.insert("action".into(), action.into());
    payload.insert("message".into(), message.into());
    if let Some(category) = category {
        payload.insert("category".into(), category.into());
    }
    let _ = ctx
        .backend
        .call("ctx_agent", serde_json::Value::Object(payload));
}

fn session_set_task(ctx: &Rc<EngineContext>, value: &str) {
    let _ = ctx.backend.call(
        "ctx_session",
        serde_json::json!({ "action": "task", "value": value }),
    );
}

fn session_add_finding(ctx: &Rc<EngineContext>, value: &str) {
    let _ = ctx.backend.call(
        "ctx_session",
        serde_json::json!({ "action": "finding", "value": value }),
    );
}

/// Session `decision` sink (open + abort narrative) → outbound `ctx_session`.
/// Headless backend → discarded BACKEND_REQUIRED envelope (#498: not rendered).
pub(crate) fn session_decision(ctx: &Rc<EngineContext>, summary: &str) {
    let _ = ctx.backend.call(
        "ctx_session",
        serde_json::json!({ "action": "decision", "value": summary }),
    );
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
            // Phase 9: human render emits a readable section heading per phase.
            if ctx.consumer_hint() == 1 {
                out.push_str(&format!("## Phase: {name}\n\n"));
            }
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
            if let Some((name, args)) = crate::parser::block::parse_directive_line(line.as_bytes())
            {
                // Phase 9: consumer=human glosses Work-directives as prose (no abort path).
                if ctx.consumer_hint() == 1 {
                    let rendered = crate::render::dispatch(ctx, &name, &args);
                    if let Some(sc) = ctx.phase_scope.borrow_mut().last_mut() {
                        sc.outputs.push((name, args.clone(), rendered.clone()));
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                } else {
                    match crate::render::dispatch_result(ctx, &name, &args) {
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
                                    cause: format!("{e}"),
                                });
                            }
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

/// Third abort sink (spec §3.5, D-10): gotcha persistence is outbound-only in
/// lean-md (no local GotchaStore — routed via ctx.backend or degraded to no-op
/// when no backend is wired). Decision: no-op for now; backend routing deferred
/// to a future task when ctx_knowledge exposes a `gotcha` action in the appendix.
fn report_phase_gotcha(_ctx: &Rc<EngineContext>, _err: &PhaseError) {}

/// Deterministic cause → GotchaCategory loose-name mapping (spec §3.5).
/// Test-only: the gotcha sink it once fed is a no-op in lean-md (outbound-only,
/// deferred). `normalize_trigger` (the sibling helper) was removed as dead code
/// in the standalone-crate decoupling (Task 6).
#[cfg(test)]
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

/// Pre-pass (Spec D-4): captures every `@phase "name" … @phase-end` block RAW in
/// `ctx.phase_bodies` so that `@dispatch phase="name"` can look it up by name.
/// Render-FREE and lifecycle-FREE (no `session_decision`, no
/// `@on complete` firing) — the Work-Bridges stay verbatim (D-3). Flat v1
/// phases: not nested; a second `@phase` before `@phase-end` does not
/// close implicitly — defensively only the first complete block per name is captured.
pub(crate) fn capture_phase_bodies(ctx: &Rc<EngineContext>, body: &str) {
    let mut open: Option<(String, Vec<&str>)> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("@phase-end") {
            if let Some((name, lines)) = open.take() {
                ctx.phase_bodies
                    .borrow_mut()
                    .entry(name)
                    .or_insert_with(|| lines.join("\n"));
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("@phase") {
            // `@phase-end` is handled above; here only real openers.
            if open.is_none() {
                open = Some((parse_phase_name(rest), Vec::new()));
            }
            continue;
        }
        if let Some((_, lines)) = open.as_mut() {
            lines.push(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::rc::Rc;

    use crate::engine::{EngineContext, render};
    use crate::header::LeanMdHeader;

    #[test]
    fn capture_phase_bodies_extracts_raw_body_without_lifecycle() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let body = "\
intro prose
@phase \"A3-parser\"
@read src/parser/block.rs
@query \"cargo nextest run\"
@phase-end
trailing prose
";
        super::capture_phase_bodies(&ctx, body);
        let got = ctx
            .phase_body("A3-parser")
            .expect("named phase body captured");
        // Raw, work-bridges verbatim, without @phase/@phase-end markers:
        assert!(got.contains("@read src/parser/block.rs"), "got: {got}");
        assert!(got.contains("@query \"cargo nextest run\""), "got: {got}");
        assert!(!got.contains("@phase"), "markers must be stripped: {got}");
        assert!(
            !got.contains("intro prose"),
            "must not leak outside the phase: {got}"
        );
        // Render-/lifecycle-free: no session-decision side-effects (no Sink ⇒ trivially none).
        assert!(ctx.phase_body("does-not-exist").is_none());
    }

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
        // A directive that returns Err headless (here @count with no pattern →
        // MissingArg) aborts its phase with the PHASE_ABORTED envelope. (@read is
        // outbound now and never Errs, so an in-process erroring directive is used.)
        let out = render("@phase \"Parser\"\n@count\nAFTER\n@phase-end\nNEXT\n");
        assert!(
            out.contains("PHASE_ABORTED \"Parser\" at @count"),
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

    // ── Sink routing tests ──────────────────────────────────────────────────
    // The local SessionCache / SinkHandles / ProjectKnowledge subsystems were
    // removed in the standalone-crate decoupling (Task 6). `@on complete` sinks
    // now route OUTBOUND through `ctx.backend.call(tool, args)`. A `RecordingBackend`
    // captures those calls so the routing remains deterministically testable
    // without `lean-ctx` on PATH.

    use crate::backend::{BackendError, CodeIntelBackend};
    use std::cell::RefCell;

    /// Test backend that records every outbound `(tool, args)` call and returns
    /// a fixed success string (so bridges treat it as a live backend, not a
    /// BACKEND_REQUIRED degradation).
    struct RecordingBackend {
        calls: std::rc::Rc<RefCell<Vec<(String, serde_json::Value)>>>,
    }

    impl CodeIntelBackend for RecordingBackend {
        fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError> {
            self.calls.borrow_mut().push((tool.to_string(), args));
            Ok(String::new())
        }
    }

    /// Build an `EngineContext` whose backend records outbound calls.
    #[allow(clippy::type_complexity)]
    fn recording_ctx(
        root: std::path::PathBuf,
    ) -> (
        Rc<EngineContext>,
        std::rc::Rc<RefCell<Vec<(String, serde_json::Value)>>>,
    ) {
        let calls = std::rc::Rc::new(RefCell::new(Vec::new()));
        let backend = Box::new(RecordingBackend {
            calls: calls.clone(),
        });
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            root,
            backend,
        ));
        (ctx, calls)
    }

    /// Find the first recorded call to `tool` whose `action` field equals `action`.
    fn find_call<'a>(
        calls: &'a [(String, serde_json::Value)],
        tool: &str,
        action: &str,
    ) -> Option<&'a serde_json::Value> {
        calls
            .iter()
            .find(|(t, a)| t == tool && a.get("action").and_then(|v| v.as_str()) == Some(action))
            .map(|(_, a)| a)
    }

    #[test]
    fn abort_reports_gotcha_with_normalized_trigger() {
        // The PHASE_ABORTED envelope is the invariant guarded here; gotcha
        // persistence is a no-op in lean-md (outbound-only, deferred).
        let root = std::env::temp_dir().join("lmd_phase_gotcha");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (ctx, _calls) = recording_ctx(root);
        // @count with no pattern → MissingArg → phase abort (in-process error;
        // @read is outbound now and never Errs).
        let out = crate::engine::render_body(&ctx, "@phase \"Parser\"\n@count\n@phase-end\n");
        assert!(
            out.contains("PHASE_ABORTED"),
            "envelope still emitted: {out}"
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
        // task + decision sinks route to outbound ctx_session calls in source order.
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let _ = crate::engine::render_body(
            &ctx,
            "@phase \"Build\"\nbody\n@on complete task=\"build done [100%]\"\n@on complete decision=\"shipped\"\n@phase-end\n",
        );
        let calls = calls.borrow();
        let task = find_call(&calls, "ctx_session", "task").expect("task sink must fire");
        assert_eq!(
            task.get("value").and_then(|v| v.as_str()),
            Some("build done [100%]"),
            "task value forwarded"
        );
        // Two decision calls are recorded: the open "Phase: Build" narrative and
        // the closing @on complete decision="shipped". Assert the latter exists.
        assert!(
            calls.iter().any(|(t, a)| t == "ctx_session"
                && a["action"] == "decision"
                && a.get("value").and_then(|v| v.as_str()) == Some("shipped")),
            "closing decision sink must forward value=shipped"
        );
        // Source order: the open `Phase: Build` decision precedes the task sink.
        let task_pos = calls
            .iter()
            .position(|(t, a)| t == "ctx_session" && a["action"] == "task")
            .unwrap();
        let dec_pos = calls
            .iter()
            .rposition(|(t, a)| t == "ctx_session" && a["action"] == "decision")
            .unwrap();
        assert!(task_pos < dec_pos, "task fires before the closing decision");
    }

    #[test]
    fn on_complete_remember_writes_knowledge() {
        // remember sink routes to an outbound ctx_knowledge remember call with the
        // slugged key, the category attr, and the content value.
        let root = std::env::temp_dir().join("lmd_oc_remember");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (ctx, calls) = recording_ctx(root);
        let _ = crate::engine::render_body(
            &ctx,
            "@phase \"P\"\nwork\n@on complete remember=\"parser uses pratt\" category=decision\n@phase-end\n",
        );
        let calls = calls.borrow();
        let k = find_call(&calls, "ctx_knowledge", "remember")
            .expect("remember sink must route to ctx_knowledge");
        assert_eq!(
            k.get("key").and_then(|v| v.as_str()),
            Some("parser_uses_pratt"),
            "key must be the slug of the content"
        );
        assert_eq!(
            k.get("category").and_then(|v| v.as_str()),
            Some("decision"),
            "category attr forwarded"
        );
        assert!(
            k.get("value")
                .and_then(|v| v.as_str())
                .is_some_and(|v| v.contains("parser uses pratt")),
            "value must contain the written content; got: {:?}",
            k.get("value")
        );
    }

    #[test]
    fn post_without_agent_degrades_to_noop() {
        // post/diary sinks route outbound to ctx_agent; the server enforces agent
        // registration. Here we assert it does not abort the phase and the call
        // is forwarded with the right action.
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let out = crate::engine::render_body(
            &ctx,
            "@phase \"P\"\nwork\n@on complete post=\"hi\" category=status\n@phase-end\n",
        );
        assert!(
            !out.contains("PHASE_ABORTED"),
            "team sink routing is not an abort: {out}"
        );
        assert!(!out.contains("panic"), "must not panic: {out}");
        let calls = calls.borrow();
        assert!(
            find_call(&calls, "ctx_agent", "post").is_some(),
            "post sink must route to ctx_agent"
        );
    }

    #[test]
    fn aborted_phase_skips_on_complete() {
        // An aborted phase must NOT fire its @on complete task sink: no outbound
        // ctx_session `task` call is recorded.
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        // @count (no pattern) Errs → phase aborts before the @on complete fires.
        let _ = crate::engine::render_body(
            &ctx,
            "@phase \"P\"\n@count\n@on complete task=\"done [100%]\"\n@phase-end\n",
        );
        let calls = calls.borrow();
        assert!(
            find_call(&calls, "ctx_session", "task").is_none(),
            "aborted phase must NOT fire its @on complete task"
        );
    }

    #[test]
    fn backend_error_in_phase_aborts_and_skips_on_complete() {
        // I2 regression: a real BackendError (Spawn/NonZero/Io) from a code-intel
        // bridge inside a @phase MUST abort the phase (Err propagates) and MUST
        // NOT fire the @on complete sink. Before the I2 fix, every bridge flattened
        // BackendError into Ok("ERROR: BACKEND_REQUIRED …"), so the phase saw no
        // error and the sink fired falsely.
        let calls = std::rc::Rc::new(RefCell::new(Vec::new()));
        struct FailingBackend {
            calls: std::rc::Rc<RefCell<Vec<(String, serde_json::Value)>>>,
        }
        impl CodeIntelBackend for FailingBackend {
            fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError> {
                self.calls.borrow_mut().push((tool.to_string(), args));
                // Mirrors a PathJail reject / headless spawn failure.
                Err(BackendError::NonZero {
                    code: 1,
                    stderr: "path outside --project-root".to_string(),
                })
            }
        }
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            std::env::temp_dir(),
            Box::new(FailingBackend {
                calls: calls.clone(),
            }),
        ));
        let out = crate::engine::render_body(
            &ctx,
            "@phase \"P\"\n@read /etc/passwd\n@on complete task=\"done [100%]\"\n@phase-end\n",
        );
        assert!(
            out.contains("PHASE_ABORTED"),
            "a real BackendError in the phase body must abort the phase: {out}"
        );
        let calls = calls.borrow();
        assert!(
            find_call(&calls, "ctx_session", "task").is_none(),
            "aborted phase (backend error) must NOT fire its @on complete task"
        );
    }

    #[test]
    fn capture_auto_emits_findings_from_body_outputs() {
        // capture=auto scans body tool outputs and routes findings to outbound
        // ctx_session `finding` calls. The @search body output is produced by the
        // recording backend; we feed a synthetic search result so auto_findings
        // extracts a finding deterministically.
        let dir = std::env::temp_dir().join("lmd_capture_auto");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hit.rs"), "fn capture_marker_77() {}\n").unwrap();

        // Backend that returns a search-result body for ctx_search so capture=auto
        // has something to extract; records ctx_session finding calls.
        let calls = std::rc::Rc::new(RefCell::new(Vec::new()));
        struct SearchBackend {
            calls: std::rc::Rc<RefCell<Vec<(String, serde_json::Value)>>>,
        }
        impl CodeIntelBackend for SearchBackend {
            fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError> {
                self.calls.borrow_mut().push((tool.to_string(), args));
                if tool == "ctx_search" {
                    Ok("src/hit.rs:1: fn capture_marker_77() {}\n1 match\n".to_string())
                } else {
                    Ok(String::new())
                }
            }
        }
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            dir.clone(),
            Box::new(SearchBackend {
                calls: calls.clone(),
            }),
        ));
        let pat = "capture_marker_77";
        let _ = crate::engine::render_body(
            &ctx,
            &format!(
                "@phase \"Scan\"\n@search {pat} {}\n@on complete capture=auto\n@phase-end\n",
                dir.to_str().unwrap()
            ),
        );
        let calls = calls.borrow();
        assert!(
            calls
                .iter()
                .any(|(t, a)| t == "ctx_session" && a["action"] == "finding"),
            "capture=auto must route body tool output to an outbound finding sink"
        );
    }

    #[test]
    fn on_complete_substitutes_call_params() {
        // @on complete inside @call substitutes params before routing outbound.
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@define close(pct, note)\n\
                   @on complete task=\"{{ note }} [{{ pct }}%]\"\n\
                   @define-end\n\n\
                   @phase \"Parser\"\n\
                   work\n\
                   @call close(100, parser fertig) /\n\
                   @phase-end\n";
        let _ = crate::engine::render_body(&ctx, doc);
        let calls = calls.borrow();
        let task = find_call(&calls, "ctx_session", "task")
            .expect("@on complete task must route outbound");
        assert_eq!(
            task.get("value").and_then(|v| v.as_str()),
            Some("parser fertig [100%]"),
            "@on complete inside @call must substitute params (D-5 composition)"
        );
    }

    #[test]
    fn phase_aborted_envelope_is_byte_stable() {
        use crate::engine::render;
        // @count (no pattern) Errs in-process → deterministic PHASE_ABORTED.
        let doc = "@phase \"X\"\n@count\n@phase-end\n";
        let a = render(doc);
        let b = render(doc);
        assert_eq!(
            a, b,
            "render output must be a deterministic function of input (#498)"
        );
        assert!(a.contains("PHASE_ABORTED \"X\" at @count"));
    }

    #[test]
    fn human_phase_emits_heading() {
        use crate::engine::render;
        let doc =
            "@lean-md\nconsumer: human\n\n@phase \"A3-parser\"\n@read src/foo.rs\n@phase-end\n";
        let out = render(doc);
        assert!(out.contains("## Phase: A3-parser"), "heading: {out}");
        assert!(
            out.contains("Read file `src/foo.rs`"),
            "body glossed: {out}"
        );
    }

    #[test]
    fn ai_phase_emits_no_heading() {
        use crate::engine::render;
        let doc = "@lean-md\nconsumer: ai\n\n@phase \"A3-parser\"\nplain line\n@phase-end\n";
        let out = render(doc);
        assert!(
            !out.contains("## Phase: A3-parser"),
            "ai: no heading: {out}"
        );
    }
}
