//! `@find` bridge → semantic / hybrid search via `ctx_semantic_search` (v1 §4.3).
//! Headless: complements the regex `@search` with semantic/hybrid retrieval.
//! `mode` (bm25|dense|hybrid) passes through; omitted → backend default (`bm25`,
//! instant, no model-load) — "erben, nicht neu erfinden" (§5). Read-only.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct FindBridge;

impl DirectiveBridge for FindBridge {
    fn name(&self) -> &'static str {
        "find"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let query = args
            .get("query")
            .or_else(|| args.positional(0))
            .ok_or(BridgeError::MissingArg("query"))?;

        // mode is optional; explicit value is validated, omitted → backend default.
        let mode = match args.get("mode") {
            Some(m @ ("bm25" | "dense" | "hybrid")) => Some(m),
            Some(other) => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @find mode '{other}'. Use: bm25|dense|hybrid"
                )))
            }
            None => None,
        };

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let path = args.get("path").unwrap_or(root);
        let top_k = args
            .get("top_k")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10); // default: 10 (matches ctx_semantic_search registered wrapper)

        Ok(crate::tools::ctx_semantic_search::handle(
            query,
            path,
            top_k,
            crate::tools::CrpMode::Off,
            None, // languages
            None, // path_glob
            mode,
            None, // workspace
            None, // artifacts
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
    fn find_is_registered() {
        assert!(super::super::default_registry().get("find").is_some());
    }

    #[test]
    fn missing_query_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = FindBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("query")), "got: {err:?}");
    }

    #[test]
    fn unknown_mode_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = FindBridge
            .execute(&ctx, &DirectiveArgs::parse("query=x mode=frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @find mode"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn bm25_query_returns_real_results_headless() {
        // bm25 is instant (no model-load) → deterministic headless result.
        let dir = std::env::temp_dir().join("lmd_find_bm25");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hit.rs"), "fn find_target_marker_42() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse("query=find_target_marker_42 mode=bm25");
        let out = FindBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "empty @find output");
        assert!(
            out.contains("find_target_marker_42") || out.contains("hit.rs"),
            "bm25 must surface the matching symbol/file, got: {out}"
        );
    }
}
