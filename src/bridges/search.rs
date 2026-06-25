//! `@search` Router bridge → the same core as the `ctx_search` MCP tool.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@search <pattern> [path=<dir>] [ext=<ext>] [max=<n>]` — defaults: path=".",
/// max=20, gitignore respected, secret paths skipped. Routes `ctx_search::handle`.
/// Phase 2 passes `path` through unchanged (PathJail inheritance is §7/Phase-7).
pub struct SearchBridge;

impl DirectiveBridge for SearchBridge {
    fn name(&self) -> &'static str {
        "search"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let pattern = args
            .positional(0)
            .or_else(|| args.get("pattern"))
            .ok_or(BridgeError::MissingArg("pattern"))?;
        let dir = args.get("path").unwrap_or(".");
        let ext = args.get("ext");
        let max = args
            .get("max")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(20);
        let mut payload = serde_json::Map::new();
        payload.insert("pattern".into(), pattern.into());
        payload.insert("path".into(), dir.into());
        if let Some(ext) = ext {
            payload.insert("ext".into(), ext.into());
        }
        payload.insert("max_results".into(), (max as u64).into());
        let out = ctx
            .backend
            .call("ctx_search", serde_json::Value::Object(payload))
            .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}"));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn searches_via_ctx_search() {
        // @search routes outbound to ctx_search. Dispatch contract: Ok(…) with the
        // live hit (lean-ctx session present) or a BACKEND_REQUIRED envelope
        // (headless / jail-refused) — never Err, never a panic.
        let dir = std::env::temp_dir().join("lmd_search_bridge");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hit.txt"), "fn target_marker_42() {}\n").unwrap();
        let args =
            DirectiveArgs::parse(&format!("target_marker_42 path={}", dir.to_str().unwrap()));
        let out = SearchBridge.execute(&ctx(), &args).unwrap();
        assert!(
            out.contains("target_marker_42") || out.contains("BACKEND_REQUIRED"),
            "got: {out}"
        );
    }

    #[test]
    fn missing_pattern_errors() {
        let err = SearchBridge
            .execute(&ctx(), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn search_is_registered() {
        assert!(super::super::default_registry().get("search").is_some());
    }
}
