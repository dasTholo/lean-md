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
                Some(sc) => finalize_phase(ctx, &mut out, sc),
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

        // @on complete (only valid inside an open phase — D-8). Accumulation in Task 7.
        if is_on_complete(trimmed) {
            if scope.is_none() {
                out.push_str("<!-- lmd: @on complete outside @phase -->\n");
            }
            // inside a phase: accumulated in Task 7; for now consumed (no render)
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
fn finalize_phase(ctx: &Rc<EngineContext>, out: &mut String, scope: PhaseScope) {
    if let Some(err) = scope.aborted.as_ref() {
        out.push_str(&err.envelope());
        out.push('\n');
        session_decision(ctx, &err.decision_summary());
        return; // skip accumulated @on complete (spec §3.2 step 2)
    }
    // clean close: fire accumulated @on complete (Task 7)
    let _ = (ctx, scope);
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
}
