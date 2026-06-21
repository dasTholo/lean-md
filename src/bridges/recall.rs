//! `@recall` bridge → durable knowledge read via `ProjectKnowledge::recall`
//! (spec §6). Symmetric pair to `@remember` — one backing (D-6). `mode` mirrors
//! `@find`'s mode resolution: omitted → `auto`; explicit `exact|semantic|hybrid`
//! validated. v1 routes every mode through the core `recall()` (exact) backing;
//! semantic/hybrid embeddings dispatch is the ctx_knowledge tool layer's job
//! (MCP path, Phase 9). Gated by sinks — headless ⇒ empty (no knowledge backing).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::core::knowledge::ProjectKnowledge;
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

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
        let _mode = resolve_recall_mode(args)?; // validated; backing is core recall() in v1
        let top_k = args.get("top_k").and_then(|s| s.parse::<usize>().ok());

        let Some(_sinks) = ctx.sinks.as_ref() else {
            return Ok(String::new()); // headless: no knowledge backing
        };
        let root = ctx.jail_root.to_str().unwrap_or(".");
        let Some(k) = ProjectKnowledge::load(root) else {
            return Ok(String::new());
        };
        let hits = k.recall(query);
        let limit = top_k.unwrap_or(hits.len());
        let mut out = String::new();
        for f in hits.iter().take(limit) {
            out.push_str(&format!("- {}\n", f.value)); // adapt field per Step 1
        }
        Ok(out)
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
    use crate::lmd::header::LeanMdHeader;
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
    fn headless_recall_is_empty() {
        let ctx = headless_ctx();
        let out = RecallBridge
            .execute(&ctx, &DirectiveArgs::parse("query=anything"))
            .unwrap();
        assert_eq!(out, "", "headless @recall has no knowledge backing");
    }
}
