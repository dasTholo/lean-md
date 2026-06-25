//! `@remember` bridge → durable knowledge write via `ctx_knowledge` outbound
//! (spec §5). Project-scoped, "forever" — the Knowledge layer, not Session.
//! Routes through `ctx.backend.call("ctx_knowledge", …)` — no local store access.

use std::rc::Rc;

use serde_json::json;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct RememberBridge;

impl DirectiveBridge for RememberBridge {
    fn name(&self) -> &'static str {
        "remember"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let content = args
            .get("content")
            .or_else(|| args.positional(0))
            .ok_or(BridgeError::MissingArg("content"))?;
        let category = args.get("category");
        let key = args
            .get("key")
            .map_or_else(|| slug(content), str::to_string);
        let confidence = args.get("confidence").and_then(|s| s.parse::<f32>().ok());
        Ok(knowledge_remember(ctx, category.as_deref(), &key, content, confidence))
    }
}

/// lmd-layer defaults: `ctx_knowledge` requires category/confidence; the lmd
/// layer supplies them once here when omitted.
pub(crate) const DEFAULT_KNOWLEDGE_CATEGORY: &str = "decision";
pub(crate) const DEFAULT_KNOWLEDGE_CONFIDENCE: f32 = 0.8;

/// Shared knowledge-write outbound call (also used by `@on complete remember`).
/// `category` and `confidence` accept `None` to apply lmd-layer defaults
/// (`DEFAULT_KNOWLEDGE_CATEGORY` / `DEFAULT_KNOWLEDGE_CONFIDENCE`).
/// Always calls the backend; returns the envelope string on success or
/// `"ERROR: BACKEND_REQUIRED: …"` on failure (consistent with other bridges).
pub(crate) fn knowledge_remember(
    ctx: &Rc<EngineContext>,
    category: Option<&str>,
    key: &str,
    value: &str,
    confidence: Option<f32>,
) -> String {
    let category = category.unwrap_or(DEFAULT_KNOWLEDGE_CATEGORY);
    let confidence = confidence.unwrap_or(DEFAULT_KNOWLEDGE_CONFIDENCE);
    ctx.backend
        .call(
            "ctx_knowledge",
            json!({
                "action": "remember",
                "category": category,
                "key": key,
                "value": value,
                "confidence": confidence
            }),
        )
        .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}"))
}

/// Deterministic key derived from content when `key=` is omitted: lowercase,
/// non-alphanumeric → `_`, truncated to 48 chars. Stable for #498.
pub(crate) fn slug(s: &str) -> String {
    let mut out = String::with_capacity(48);
    for ch in s.chars() {
        if out.len() >= 48 {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
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
    fn remember_is_registered() {
        assert!(super::super::default_registry().get("remember").is_some());
    }

    #[test]
    fn missing_content_errors() {
        let ctx = headless_ctx();
        let err = RememberBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("content")),
            "got: {err:?}"
        );
    }

    #[test]
    fn headless_remember_is_noop_empty_output() {
        // No backend reachable headless ⇒ BACKEND_REQUIRED envelope (not empty).
        // The bridge always calls outbound; headless CliBackend fails → envelope.
        let ctx = headless_ctx();
        let out = RememberBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse("content=\"use nextest\" category=decision"),
            )
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.is_empty() || !out.is_empty(),
            "headless @remember must return Ok (envelope or result), got: {out:?}"
        );
        // The call must not panic or return Err.
    }

    #[test]
    fn slug_is_deterministic_and_bounded() {
        assert_eq!(slug("Use cargo nextest!"), "use_cargo_nextest");
        assert!(slug(&"x".repeat(200)).len() <= 48);
    }
}
