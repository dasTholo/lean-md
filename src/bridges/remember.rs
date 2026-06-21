//! `@remember` bridge → durable knowledge write via `ProjectKnowledge::remember`
//! (spec §5). Project-scoped, "forever" — the Knowledge layer, not Session.
//! Gated by `EngineContext.sinks`: headless ⇒ no-op (deterministic golden output).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::core::knowledge::ProjectKnowledge;
use crate::core::memory_policy::MemoryPolicy;
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

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
        knowledge_remember(ctx, category, &key, content, confidence)?;
        // No visible render body: the write is a side effect (determinism #498).
        Ok(String::new())
    }
}

/// lmd-layer defaults: `ProjectKnowledge::remember` has no backend default for
/// category/confidence (both required), so the lmd sink supplies them once here.
pub(crate) const DEFAULT_KNOWLEDGE_CATEGORY: &str = "decision";
pub(crate) const DEFAULT_KNOWLEDGE_CONFIDENCE: f32 = 0.8;

/// Shared knowledge-write sink (also used by `@on complete remember`).
/// `category` and `confidence` accept `None` to apply lmd-layer defaults
/// (`DEFAULT_KNOWLEDGE_CATEGORY` / `DEFAULT_KNOWLEDGE_CONFIDENCE`).
/// Gated: no sinks ⇒ no-op. Loads-by-root, merges, persists. Contradiction
/// handling is delegated to `remember` (spec §5: passed through, not duplicated).
pub(crate) fn knowledge_remember(
    ctx: &Rc<EngineContext>,
    category: Option<&str>,
    key: &str,
    value: &str,
    confidence: Option<f32>,
) -> Result<(), BridgeError> {
    let category = category.unwrap_or(DEFAULT_KNOWLEDGE_CATEGORY);
    let confidence = confidence.unwrap_or(DEFAULT_KNOWLEDGE_CONFIDENCE);
    let Some(sinks) = ctx.sinks.as_ref() else {
        return Ok(()); // headless: no-op degradation (spec §7)
    };
    let root = ctx.jail_root.to_str().unwrap_or(".");
    let mut k = ProjectKnowledge::load(root).unwrap_or_else(|| ProjectKnowledge::new(root));
    let policy = MemoryPolicy::default();
    k.remember(category, key, value, &sinks.session_id, confidence, &policy);
    k.save().map_err(BridgeError::Io)?;
    Ok(())
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
    use crate::lmd::header::LeanMdHeader;
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
        // No sinks ⇒ no store write, empty deterministic output.
        let ctx = headless_ctx();
        let out = RememberBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse("content=\"use nextest\" category=decision"),
            )
            .unwrap();
        assert_eq!(out, "", "headless @remember must render nothing");
    }

    #[test]
    fn slug_is_deterministic_and_bounded() {
        assert_eq!(slug("Use cargo nextest!"), "use_cargo_nextest");
        assert!(slug(&"x".repeat(200)).len() <= 48);
    }
}
