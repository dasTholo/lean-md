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
    let lines = body.lines();

    for line in lines {
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
            // Task 4: render the phase body as plain markdown (per-directive
            // dispatch + abort arrives in Task 5). Accumulate into the scope's
            // body buffer reusing `outputs` is premature; render inline.
            sc.outputs.push((String::new(), String::new())); // placeholder removed in Task 5
            out.push_str(&render_markdown(ctx, line));
            out.push('\n');
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

/// Clean phase close. Task 4: no accumulated sinks yet (fires in Task 7);
/// aborted phases are handled in Task 5.
fn finalize_phase(_ctx: &Rc<EngineContext>, _out: &mut String, _scope: PhaseScope) {}

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
}
