//! `@symbol` Router bridge → read-only code navigation/structure over the
//! lean-ctx `ctx_refactor` LSP/IDE actions (spec §4.2). Six ops:
//! refs/def/impl/declaration/type-hierarchy/overview. The token-bearing
//! property (spec §4.2) is cache-name enrichment: ctx_refactor returns bare
//! `file:line:col`; this bridge reads the target line from the shared
//! EngineContext cache (warm ~13-tok hit, §3.4) and appends the extracted
//! type/symbol name — in Rust, zero agent-context tokens. `declaration` and
//! `type-hierarchy` are IDE-only (§3.3): the BACKEND_REQUIRED envelope passes
//! through unchanged.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct SymbolBridge;

/// Map a user-facing op alias to the ctx_refactor action name. None = unknown.
fn map_op(op: &str) -> Option<&'static str> {
    Some(match op {
        "refs" | "references" => "references",
        "def" | "definition" => "definition",
        "impl" | "implementations" => "implementations",
        "declaration" => "declaration",
        "type-hierarchy" | "type_hierarchy" => "type_hierarchy",
        "overview" | "symbols_overview" => "symbols_overview",
        _ => return None,
    })
}

impl DirectiveBridge for SymbolBridge {
    fn name(&self) -> &'static str {
        "symbol"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
        let action = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @symbol op '{op}'. Use: refs|def|impl|declaration|type-hierarchy|overview"
            ))
        })?;
        let root = ctx.jail_root.to_str().unwrap_or(".").to_string();

        if action == "symbols_overview" {
            return overview(ctx, args, &root);
        }
        // Position-based ops land in Task 2.
        Err(BridgeError::Resolve(format!(
            "@symbol {op} not yet implemented"
        )))
    }
}

/// `@symbol overview <path>` → ctx_refactor symbols_overview (path only).
fn overview(
    _ctx: &Rc<EngineContext>,
    args: &DirectiveArgs,
    root: &str,
) -> Result<String, BridgeError> {
    let path = args
        .positional(1)
        .or_else(|| args.get("path"))
        .ok_or(BridgeError::MissingArg("path"))?;
    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;
    let obj = serde_json::json!({ "action": "symbols_overview" });
    Ok(crate::tools::ctx_refactor::handle(&obj, root, &abs))
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
    fn op_aliases_map_to_ctx_refactor_actions() {
        assert_eq!(map_op("refs"), Some("references"));
        assert_eq!(map_op("def"), Some("definition"));
        assert_eq!(map_op("impl"), Some("implementations"));
        assert_eq!(map_op("declaration"), Some("declaration"));
        assert_eq!(map_op("type-hierarchy"), Some("type_hierarchy"));
        assert_eq!(map_op("overview"), Some("symbols_overview"));
        assert_eq!(map_op("frobnicate"), None);
    }

    #[test]
    fn unknown_op_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate x.rs"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @symbol op"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn missing_op_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("op")), "got: {err:?}");
    }

    #[test]
    fn overview_routes_on_a_real_rust_file() {
        // Headless tree-sitter symbols_overview on a real symbol — never a panic.
        let dir = std::env::temp_dir().join("lmd_symbol_overview");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha() {}\npub struct Beta;\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("overview {}", f.to_str().unwrap()));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        // Either an overview listing or a precise degradation envelope.
        assert!(!out.trim().is_empty(), "overview produced empty output");
        assert!(
            out.contains("alpha")
                || out.contains("Beta")
                || out.contains("ERROR")
                || out.contains("No results")
                || out.contains("No symbols"),
            "unexpected overview output: {out}"
        );
    }

    #[test]
    fn overview_missing_path_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("overview"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("path")),
            "got: {err:?}"
        );
    }

    #[test]
    fn symbol_is_registered() {
        assert!(super::super::default_registry().get("symbol").is_some());
    }
}
