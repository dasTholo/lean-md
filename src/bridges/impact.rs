//! `@impact` bridge → blast-radius / dependency chain via `ctx_impact` (v1 §4.3).
//! Headless risk-gate before edits. Read-only. Directive surface is the honest
//! scope `analyze|chain` (the backend supports more actions, not exposed here).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct ImpactBridge;

impl DirectiveBridge for ImpactBridge {
    fn name(&self) -> &'static str {
        "impact"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        // action is positional-0 or action=; default analyze (= risk-gate default).
        let action = match args.positional(0).or_else(|| args.get("action")).unwrap_or("analyze") {
            a @ ("analyze" | "chain") => a,
            other => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @impact action '{other}'. Use: analyze|chain"
                )))
            }
        };

        // both analyze and chain target a symbol/file → path required.
        let path = args
            .get("path")
            .or_else(|| args.positional(1))
            .ok_or(BridgeError::MissingArg("path"))?;
        let depth = args.get("depth").and_then(|s| s.parse::<usize>().ok());

        let root = ctx.jail_root.to_str().unwrap_or(".");
        Ok(crate::tools::ctx_impact::handle(
            action,
            Some(path),
            root,
            depth,
            None, // format: backend default
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
    fn impact_is_registered() {
        assert!(super::super::default_registry().get("impact").is_some());
    }

    #[test]
    fn unknown_action_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = ImpactBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate path=x.rs"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @impact action"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn missing_path_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = ImpactBridge
            .execute(&ctx, &DirectiveArgs::parse("analyze"))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("path")), "got: {err:?}");
    }

    #[test]
    fn analyze_dispatches_headless() {
        let dir = std::env::temp_dir().join("lmd_impact_analyze");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("m.rs"), "fn impacted() {}\nfn caller() { impacted(); }\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse("analyze path=m.rs");
        let out = ImpactBridge.execute(&ctx, &args).unwrap();
        // Dispatch must produce output (impact data or a clear "no data" message),
        // never the unknown-action error and never a panic.
        assert!(!out.trim().is_empty(), "empty @impact output");
        assert!(!out.contains("Unknown action"), "must not hit the backend unknown-action path: {out}");
    }
}
