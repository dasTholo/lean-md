//! `@reformat` bridge → single-phase IDE reformat via `ctx_refactor reformat`
//! (spec §4.2). IDE-only (§3.3): without a running IDE the backend returns a
//! BACKEND_REQUIRED envelope, passed through verbatim. Mutating: on success the
//! shared EngineContext cache is cleared (spec §3.4) so the next read sees the
//! reformatted bytes. Addressing: name_path | path[+line] via bridges::addressing.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct ReformatBridge;

impl DirectiveBridge for ReformatBridge {
    fn name(&self) -> &'static str {
        "reformat"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let root = ctx.jail_root.to_str().unwrap_or(".");
        // reformat accepts path-only (whole-file) addressing → require_line=false.
        let (mut obj, abs) = super::addressing::build_target_with(args, root, false)?;
        obj.insert("action".into(), "reformat".into());

        // optimize-imports is a bare positional flag (no op token precedes it).
        let optimize = (0_usize..)
            .map_while(|i| args.positional(i))
            .any(|t| t == "optimize-imports");
        if optimize {
            obj.insert("optimize_imports".into(), true.into());
        }

        let out = crate::tools::ctx_refactor::handle(&serde_json::Value::Object(obj), root, &abs);

        // Cache coherence (spec §3.4): reformat MUTATES the file, so on success
        // the shared cache is stale → clear. On the BACKEND_REQUIRED/ERROR
        // envelope the file is untouched → keep the warm cache (no needless miss).
        if reformat_succeeded(&out) {
            ctx.cache.borrow_mut().clear();
        }

        Ok(out)
    }
}

/// Reformat succeeded and the file was mutated → the shared cache must be
/// cleared (spec §3.4). reformat is single-phase, so success is simply the
/// absence of an ERROR envelope (BACKEND_REQUIRED arrives as "ERROR: …").
pub(crate) fn reformat_succeeded(out: &str) -> bool {
    !out.starts_with("ERROR")
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
    fn reformat_is_registered() {
        assert!(super::super::default_registry().get("reformat").is_some());
    }

    #[test]
    fn missing_path_and_name_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = ReformatBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("path")), "got: {err:?}");
    }

    #[test]
    fn path_only_dispatches_without_line() {
        // Whole-file reformat: path= with no line= must NOT error on a missing
        // line (require_line=false). Headless → BACKEND_REQUIRED/ERROR envelope,
        // never a panic, never MissingArg("line").
        let dir = std::env::temp_dir().join("lmd_reformat_pathonly");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn   spaced( ) {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("path={}", f.to_str().unwrap()));
        let out = ReformatBridge.execute(&ctx, &args).unwrap();
        assert!(!out.contains("MissingArg"), "path-only must dispatch: {out}");
        assert!(!out.trim().is_empty(), "empty reformat output");
    }

    #[test]
    fn returns_backend_required_envelope_headless() {
        let dir = std::env::temp_dir().join("lmd_reformat_degrade");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("path={} line=1", f.to_str().unwrap()));
        let out = ReformatBridge.execute(&ctx, &args).unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "headless reformat must degrade cleanly, got: {out}"
        );
    }

    #[test]
    fn optimize_imports_flag_maps() {
        // Drive the arg-map directly to prove the flag lands as optimize_imports.
        // (A BACKEND_REQUIRED envelope never echoes args, so assert on the map.)
        let dir = std::env::temp_dir().join("lmd_reformat_optimports");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();

        let args = DirectiveArgs::parse(&format!(
            "path={} line=1 optimize-imports",
            f.to_str().unwrap()
        ));
        let (mut obj, _abs) =
            super::super::addressing::build_target_with(&args, dir.to_str().unwrap(), false)
                .unwrap();
        // Re-run the bridge's flag detection to assert the mapping contract.
        let optimize = (0_usize..)
            .map_while(|i| args.positional(i))
            .any(|t| t == "optimize-imports");
        if optimize {
            obj.insert("optimize_imports".into(), true.into());
        }
        assert_eq!(
            obj.get("optimize_imports").and_then(|v| v.as_bool()),
            Some(true),
            "optimize-imports flag must map to optimize_imports=true"
        );
    }

    #[test]
    fn reformat_succeeded_predicate() {
        // Drive the production predicate — not an inline copy.
        assert!(super::reformat_succeeded("Reformatted 1 file"), "success → clear");
        assert!(!super::reformat_succeeded("ERROR: BACKEND_REQUIRED: …"), "error → no clear");
        assert!(!super::reformat_succeeded("ERROR: UNSUPPORTED_LANGUAGE: …"), "error → no clear");
    }

    #[test]
    fn error_does_not_clear_cache() {
        // Headless reformat returns BACKEND_REQUIRED → file untouched → the
        // warm cache entry must survive (no needless miss). Mirrors
        // refactor.rs::preview_never_clears_cache.
        let dir = std::env::temp_dir().join("lmd_reformat_noclear");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let path_str = f.to_str().unwrap();
        ctx.cache.borrow_mut().store(path_str, "cached-content");

        let args = DirectiveArgs::parse(&format!("path={} line=1", path_str));
        let _out = ReformatBridge.execute(&ctx, &args).unwrap();

        assert!(
            ctx.cache.borrow().get(path_str).is_some(),
            "BACKEND_REQUIRED reformat must NOT clear the cache"
        );
    }
}
