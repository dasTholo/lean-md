//! `@architecture` bridge → project structure views via `ctx_architecture` (v1 §4.3).
//! Headless orientation directive for large plans. Read-only. Honest scope:
//! overview|clusters|layers|cycles|hotspots (positional-0, default overview).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct ArchitectureBridge;

impl DirectiveBridge for ArchitectureBridge {
    fn name(&self) -> &'static str {
        "architecture"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let action = match args
            .positional(0)
            .or_else(|| args.get("action"))
            .unwrap_or("overview")
        {
            a @ ("overview" | "clusters" | "layers" | "cycles" | "hotspots") => a,
            other => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @architecture view '{other}'. Use: overview|clusters|layers|cycles|hotspots"
                )));
            }
        };

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let path = args.get("path"); // optional sub-scope

        Ok(crate::tools::ctx_architecture::handle(
            action, path, root, None, // format: backend default
        ))
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
    fn architecture_is_registered() {
        assert!(
            super::super::default_registry()
                .get("architecture")
                .is_some()
        );
    }

    #[test]
    fn unknown_view_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = ArchitectureBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => {
                assert!(m.contains("unknown @architecture view"), "got: {m}");
            }
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn overview_is_the_default_and_dispatches() {
        let dir = std::env::temp_dir().join("lmd_arch_overview");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "pub mod x; pub fn arch_anchor() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        // No positional → defaults to overview; must dispatch (non-empty), not error.
        let out = ArchitectureBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap();
        assert!(!out.trim().is_empty(), "empty @architecture output");
    }
}
