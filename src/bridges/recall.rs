//! `@recall` bridge → durable knowledge read via `ctx_knowledge` outbound
//! (spec §6). Symmetric pair to `@remember` — one backing (D-6). `mode` mirrors
//! `@find`'s mode resolution: omitted → `auto`; explicit `exact|semantic|hybrid`
//! validated. Routes to `ctx.backend.call("ctx_knowledge", {"action":"recall",…})`;
//! semantic/hybrid dispatch is the ctx_knowledge tool layer's job (Phase 9).

use std::rc::Rc;

use serde_json::json;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct RecallBridge;

impl DirectiveBridge for RecallBridge {
    fn name(&self) -> &'static str {
        "recall"
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
        let mode = resolve_recall_mode(args)?;
        let top_k = args.get("top_k").and_then(|s| s.parse::<usize>().ok());

        let mut call_args = json!({
            "action": "recall",
            "query": query,
            "mode": mode
        });
        if let Some(k) = top_k {
            call_args["top_k"] = json!(k);
        }

        Ok(ctx
            .backend
            .call("ctx_knowledge", call_args)
            .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
    }
}

/// Resolve `@recall` mode. Omitted → `auto` (config-inherited, D-6); explicit
/// `exact|semantic|hybrid` validated. Unknown → clear error. Mirrors
/// `find.rs::resolve_mode` exactly (spec §6, §9.5).
fn resolve_recall_mode(args: &DirectiveArgs) -> Result<&str, BridgeError> {
    match args.get("mode") {
        Some(m @ ("exact" | "semantic" | "hybrid" | "auto")) => Ok(m),
        Some(other) => Err(BridgeError::Resolve(format!(
            "unknown @recall mode '{other}'. Use: exact|semantic|hybrid|auto"
        ))),
        None => Ok("auto"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn headless_ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn recall_is_registered() {
        assert!(super::super::default_registry().get("recall").is_some());
    }

    #[test]
    fn missing_query_errors() {
        let ctx = headless_ctx();
        let err = RecallBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("query")),
            "got: {err:?}"
        );
    }

    #[test]
    fn omitted_mode_defaults_to_auto() {
        assert_eq!(
            resolve_recall_mode(&DirectiveArgs::parse("query=x")).unwrap(),
            "auto"
        );
        assert_eq!(
            resolve_recall_mode(&DirectiveArgs::parse("query=x mode=exact")).unwrap(),
            "exact"
        );
        assert!(resolve_recall_mode(&DirectiveArgs::parse("query=x mode=bad")).is_err());
    }

    #[test]
    fn headless_recall_returns_backend_envelope_or_empty() {
        // No backend reachable headless ⇒ BACKEND_REQUIRED envelope.
        // `@recall` is a read directive; contract: Ok(string) where string is
        // either empty OR contains "BACKEND"/"ERROR" prefix (no arbitrary text).
        let ctx = headless_ctx();
        let out = RecallBridge
            .execute(&ctx, &DirectiveArgs::parse("query=anything"))
            .unwrap();
        assert!(
            out.is_empty() || out.contains("BACKEND") || out.contains("ERROR"),
            "headless @recall must return empty or BACKEND/ERROR envelope, got: {out:?}"
        );
    }
}
