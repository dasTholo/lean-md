//! `@inspect` bridge → IDE inspections via `ctx_refactor inspections` (spec §4.2).
//! Read-only: NEVER clears the cache. IDE-only (§3.3): without a running IDE the
//! backend returns a BACKEND_REQUIRED envelope, passed through verbatim.
//! mode=run (default) = diagnostics for one file; mode=list = enabled inspections
//! of the current project profile (project-wide, no path).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct InspectBridge;

impl DirectiveBridge for InspectBridge {
    fn name(&self) -> &'static str {
        "inspect"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        // mode is positional-0; default "run" (= ctx_refactor default; §5).
        let mode = match args.positional(0).unwrap_or("run") {
            m @ ("run" | "list") => m,
            other => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @inspect mode '{other}'. Use: run|list"
                )));
            }
        };

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let mut obj = serde_json::Map::new();
        obj.insert("action".into(), "inspections".into());
        obj.insert("mode".into(), mode.into());

        // run = diagnostics for one file (needs path); list = project-wide (no path).
        let abs = if mode == "run" {
            let path = args
                .positional(1)
                .or_else(|| args.get("path"))
                .ok_or(BridgeError::MissingArg("path"))?;
            crate::pathx::resolve_tool_path(Some(root), None, path)
                .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?
        } else {
            String::new()
        };

        // Read-only: backend call only, no cache mutation.
        if !abs.is_empty() {
            obj.insert("path".into(), abs.clone().into());
        }
        let out = ctx
            .backend
            .call("ctx_refactor", serde_json::Value::Object(obj))
            .map_err(BridgeError::Backend)?;
        Ok(out)
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
    fn inspect_is_registered() {
        assert!(super::super::default_registry().get("inspect").is_some());
    }

    #[test]
    fn unknown_mode_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = InspectBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate x.rs"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @inspect mode"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn run_without_path_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = InspectBridge
            .execute(&ctx, &DirectiveArgs::parse("run"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("path")),
            "got: {err:?}"
        );
    }

    #[test]
    fn run_dispatches_with_path_headless() {
        let dir = std::env::temp_dir().join("lmd_inspect_run");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("i.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("run {}", f.to_str().unwrap()));
        let out = InspectBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "empty inspect output");
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "run must degrade to BACKEND_REQUIRED or ERROR headless, got: {out}"
        );
    }

    #[test]
    fn list_dispatches_without_path() {
        // list is project-wide: no path required, must not MissingArg.
        let ctx = ctx_at(std::env::temp_dir());
        let out = InspectBridge
            .execute(&ctx, &DirectiveArgs::parse("list"))
            .unwrap();
        assert!(!out.contains("MissingArg"), "list needs no path: {out}");
        assert!(!out.trim().is_empty(), "empty list output");
    }

    // run_never_clears_cache removed: ctx.cache field removed in Task 6.5;
    // cache coherence is now owned by the backend.
}
