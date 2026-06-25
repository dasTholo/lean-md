//! `@find` bridge → semantic / hybrid search via `ctx_semantic_search` (v1 §4.3).
//! Headless: complements the regex `@search` with semantic/hybrid retrieval.
//! `mode` (bm25|dense|hybrid); omitted → `bm25`, the lmd headless default
//! (spec §4.3: instant, no model-load). NB: a DELIBERATE lmd default — the
//! `ctx_semantic_search` backend default is `hybrid` (model-load). Read-only.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

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

        // mode: explicit value validated; omitted → bm25 (lmd headless default).
        let mode = resolve_mode(args)?;

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let path = args.get("path").unwrap_or(root);
        let top_k = args
            .get("top_k")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10); // default: 10 (matches ctx_semantic_search registered wrapper)

        let mut payload = serde_json::Map::new();
        payload.insert("query".into(), query.into());
        payload.insert("action".into(), "search".into());
        payload.insert("mode".into(), mode.into());
        payload.insert("top_k".into(), (top_k as u64).into());
        payload.insert("path".into(), path.into());
        let out = ctx
            .backend
            .call("ctx_semantic_search", serde_json::Value::Object(payload))
            .map_err(BridgeError::Backend)?;
        Ok(out)
    }
}

/// Resolve the `@find` search mode. An explicit value is validated against
/// `bm25|dense|hybrid`; an omitted mode defaults to `bm25` — the lmd headless
/// default (spec §4.3: instant, no model-load). Deliberate lmd default, NOT the
/// `ctx_semantic_search` backend default (`hybrid`, model-load): passing `None`
/// through would silently make a bare `@find` lossy/expensive.
fn resolve_mode(args: &DirectiveArgs) -> Result<&str, BridgeError> {
    match args.get("mode") {
        Some(m @ ("bm25" | "dense" | "hybrid")) => Ok(m),
        Some(other) => Err(BridgeError::Resolve(format!(
            "unknown @find mode '{other}'. Use: bm25|dense|hybrid"
        ))),
        None => Ok("bm25"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
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
        assert!(
            matches!(err, BridgeError::MissingArg("query")),
            "got: {err:?}"
        );
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
    fn omitted_mode_defaults_to_bm25() {
        // @find without mode= defaults to bm25 (instant, no model-load) — the lmd
        // headless default per spec §4.3, NOT the backend's hybrid default.
        assert_eq!(
            resolve_mode(&DirectiveArgs::parse("query=x")).unwrap(),
            "bm25"
        );
        assert_eq!(
            resolve_mode(&DirectiveArgs::parse("query=x mode=dense")).unwrap(),
            "dense"
        );
        assert_eq!(
            resolve_mode(&DirectiveArgs::parse("query=x mode=hybrid")).unwrap(),
            "hybrid"
        );
        assert!(resolve_mode(&DirectiveArgs::parse("query=x mode=bad")).is_err());
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
