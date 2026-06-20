//! `@repomap` bridge → PageRank repo map via `ctx_repomap` (v1 §4.3).
//! Headless orientation directive for large plans (controller contract). Read-only.
//! `focus=a.rs,b.rs` (optional, comma-separated, surrounding [] tolerated) biases
//! the ranking; `max_tokens=N` fits the output to a budget.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct RepomapBridge;

impl DirectiveBridge for RepomapBridge {
    fn name(&self) -> &'static str {
        "repomap"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let root = ctx.jail_root.to_str().unwrap_or(".");
        let max_tokens = args
            .get("max_tokens")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(2048); // default: matches ctx_repomap registered DEFAULT_MAX_TOKENS

        let focus: Vec<String> = args
            .get("focus")
            .map(|s| {
                s.trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // session_files: none threaded from the engine in this phase (YAGNI).
        Ok(crate::tools::ctx_repomap::handle(
            root,
            max_tokens,
            &focus,
            &[],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    #[test]
    fn repomap_is_registered() {
        assert!(super::super::default_registry().get("repomap").is_some());
    }

    #[test]
    fn ranks_symbols_headless() {
        let dir = std::env::temp_dir().join("lmd_repomap_rank");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn repo_anchor_alpha() {}\n").unwrap();
        std::fs::write(
            dir.join("b.rs"),
            "fn caller() { super_unused(); }\nfn super_unused() {}\n",
        )
        .unwrap();
        let ctx = ctx_at(dir.clone());

        let out = RepomapBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap();
        assert!(!out.trim().is_empty(), "empty @repomap output");
        // The map either ranks symbols or reports an empty index — never a panic.
        assert!(
            out.contains("repo_anchor_alpha") || out.contains("No "),
            "repomap must rank symbols or report an empty index, got: {out}"
        );
    }

    #[test]
    fn focus_list_parses_without_error() {
        let dir = std::env::temp_dir().join("lmd_repomap_focus");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn focused_fn() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        // Bracketed + comma-separated focus must parse and dispatch (no error).
        let out = RepomapBridge
            .execute(&ctx, &DirectiveArgs::parse("focus=[a.rs] max_tokens=500"))
            .unwrap();
        assert!(
            !out.trim().is_empty(),
            "empty @repomap output for focus form"
        );
    }
}
