//! `@review` bridge → automated code review via `ctx_review` (v1 §4.4).
//! Headless. Read-only. `action` (review|diff-review|checklist) is positional-0,
//! default `review`. `review` is COMPOSITE: the backend fuses impact + caller
//! tracking + smells + test-discovery into one verdict (the HOW-seam for
//! `requesting-code-review`-style skills). Path semantics branch per action:
//!   review/checklist → `path=` is an FS path (jail-resolved);
//!   diff-review     → `diff=` (or positional-1) is RAW diff text, passed
//!                     verbatim (no git call, NOT jail-resolved — the backend
//!                     parses `+++ b/`/`diff --git a/`).
//! `depth=` passes through (backend default 3). A multi-line diff cannot ride a
//! single-line directive arg → standalone diff-review is for the Phase-4 pipe
//! (`@query git diff | @review diff-review`); Phase 4 wires the pipe:
//! `piped_input()` carries the diff; the standalone `diff=` arg remains a fallback.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct ReviewBridge;

impl DirectiveBridge for ReviewBridge {
    fn name(&self) -> &'static str {
        "review"
    }

    fn accepts_pipe(&self) -> bool {
        true
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let action = match args
            .positional(0)
            .or_else(|| args.get("action"))
            .unwrap_or("review")
        {
            a @ ("review" | "diff-review" | "checklist") => a,
            other => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @review action '{other}'. Use: review|diff-review|checklist"
                )));
            }
        };

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let depth = args.get("depth").and_then(|s| s.parse::<usize>().ok());

        // diff-review: raw diff text (verbatim, no git, no jail).
        // review/checklist: FS path → jail-resolve when present.
        let path_arg: Option<String> = match action {
            "diff-review" => args
                .piped_input()
                .or_else(|| args.get("diff"))
                .or_else(|| args.positional(1))
                .map(str::to_string),
            _ => match args.get("path") {
                Some(p) => Some(
                    crate::pathx::resolve_tool_path(Some(root), None, p)
                        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?,
                ),
                None => None,
            },
        };

        let mut payload = serde_json::Map::new();
        payload.insert("action".into(), action.into());
        if let Some(ref p) = path_arg {
            payload.insert("path".into(), p.clone().into());
        }
        if let Some(d) = depth {
            payload.insert("depth".into(), (d as u64).into());
        }
        let out = ctx
            .backend
            .call("ctx_review", serde_json::Value::Object(payload))
            .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}"));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tokens::count_tokens;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    #[test]
    fn review_is_registered() {
        assert!(super::super::default_registry().get("review").is_some());
    }

    #[test]
    fn review_unknown_action_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = ReviewBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => {
                assert!(m.contains("unknown @review action"), "got: {m}");
                assert!(
                    m.contains("Use: review|diff-review|checklist"),
                    "missing hint: {m}"
                );
            }
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn review_checklist_dispatches_headless() {
        // checklist is project-wide (no path) → a real, non-empty dispatch.
        let ctx = ctx_at(std::env::temp_dir());
        let out = ReviewBridge
            .execute(&ctx, &DirectiveArgs::parse("checklist"))
            .unwrap();
        assert!(!out.trim().is_empty(), "empty @review checklist output");
    }

    #[test]
    fn diff_review_is_faithful_passthrough_and_dense() {
        // diff-review takes RAW diff text (no git, no jail). A single-quoted arg
        // preserves real newlines verbatim (args.rs tokenizer). Build a bulky
        // 2-file diff; the verdict must (a) surface the changed files and (b) be
        // DENSER (fewer cl100k tokens) than the raw diff — the §4.4 output-
        // compression claim, NOT a structural-lever claim.
        let ctx = ctx_at(std::env::temp_dir().join("lmd_review_diff"));

        // A normal multi-line Rust string literal (real newlines), no single
        // quotes inside. 24 added lines per file → bulky raw input.
        let mut body = String::new();
        body.push_str("diff --git a/src/alpha.rs b/src/alpha.rs\n");
        body.push_str("--- a/src/alpha.rs\n+++ b/src/alpha.rs\n@@ -1,1 +1,25 @@\n");
        for i in 0..24 {
            body.push_str(&format!("+fn alpha_added_{i}() -> u32 {{ {i} }}\n"));
        }
        body.push_str("diff --git a/src/beta.rs b/src/beta.rs\n");
        body.push_str("--- a/src/beta.rs\n+++ b/src/beta.rs\n@@ -1,1 +1,25 @@\n");
        for i in 0..24 {
            body.push_str(&format!("+fn beta_added_{i}() -> u32 {{ {i} }}\n"));
        }

        // Single-quoted arg → verbatim (real newlines preserved, no escaping).
        let directive = format!("diff-review diff='{body}'");
        let parsed = DirectiveArgs::parse(&directive);
        let raw_diff = parsed
            .get("diff")
            .expect("diff arg must round-trip through the single-quote tokenizer");

        let out = ReviewBridge.execute(&ctx, &parsed).unwrap();

        assert!(!out.trim().is_empty(), "empty @review diff-review output");
        assert!(
            out.contains("alpha.rs") && out.contains("beta.rs"),
            "diff-review must surface both changed files, got: {out}"
        );
        assert!(
            count_tokens(&out) < count_tokens(raw_diff),
            "diff-review verdict ({} tok) must be denser than the raw diff ({} tok)",
            count_tokens(&out),
            count_tokens(raw_diff)
        );
    }

    #[test]
    fn review_accepts_pipe() {
        assert!(
            ReviewBridge.accepts_pipe(),
            "@review must accept piped diff"
        );
    }

    #[test]
    fn diff_review_reads_piped_input() {
        // The pipe supplies the diff text; no `diff=` arg present.
        let ctx = ctx_at(std::env::temp_dir().join("lmd_review_pipe"));
        let mut diff = String::from("diff --git a/src/p.rs b/src/p.rs\n");
        diff.push_str("--- a/src/p.rs\n+++ b/src/p.rs\n@@ -1,1 +1,3 @@\n");
        for i in 0..6 {
            diff.push_str(&format!("+fn piped_added_{i}() {{}}\n"));
        }
        let args = DirectiveArgs::parse("diff-review").with_piped_input(diff);
        let out = ReviewBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "empty piped diff-review output");
        assert!(
            out.contains("p.rs"),
            "must surface the changed file, got: {out}"
        );
    }
}
