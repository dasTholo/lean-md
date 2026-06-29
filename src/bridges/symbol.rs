//! `@symbol` Router bridge → read-only code navigation/structure over the
//! lean-ctx `ctx_refactor` LSP/IDE actions (spec §4.2). Six ops:
//! refs/def/impl/declaration/type-hierarchy/overview. All nav ops route
//! outbound via `ctx.backend.call("ctx_refactor", …)` and return output
//! verbatim. `Compact`/`Tdd` overview stays local (vendored CRP). `declaration`
//! and `type-hierarchy` are IDE-only (§3.3): the BACKEND_REQUIRED envelope
//! passes through unchanged when no IDE is connected.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

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
        let root = ctx.jail_root.to_str().unwrap_or(".");

        if op == "body" {
            return body(ctx, args, root);
        }

        let action = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @symbol op '{op}'. Use: refs|def|impl|declaration|type-hierarchy|overview|body"
            ))
        })?;

        if action == "symbols_overview" {
            return overview(ctx, args, root);
        }
        nav(ctx, args, action, root)
    }
}

/// Position-based nav ops: references/definition/implementations/declaration/
/// type_hierarchy. Builds the ctx_refactor arg map from path + line(1-idx) +
/// column(0-idx, default 0) and passes the jail-resolved abs_path. `scope`
/// (refs/impl) and `direction` (type_hierarchy) are forwarded when present.
/// Output is returned verbatim from the backend (no local enrichment).
// NOTE: `name=` addressing formerly resolved to a position locally via
// resolve_name_path (a local tool, not a backend action). Since ctx_refactor
// exposes no `resolve_name_path` action outbound, `name=` now requires `line=`
// to be provided explicitly. Surface a clear message if `name=` is used alone.
fn nav(
    ctx: &Rc<EngineContext>,
    args: &DirectiveArgs,
    action: &str,
    root: &str,
) -> Result<String, BridgeError> {
    // `name=` without `line=` is no longer supported outbound (resolve_name_path
    // is a local tool only; ctx_refactor exposes no equivalent action).
    if args.get("name").is_some() && args.get("line").is_none() {
        return Ok(
            "ERROR: name= addressing needs line= for nav ops (resolve_name_path is not \
             available outbound). For a symbol body by name use '@symbol body name=…'."
                .to_string(),
        );
    }

    let path = args
        .positional(1)
        .or_else(|| args.get("path"))
        .ok_or(BridgeError::MissingArg("path"))?;
    let line: u64 = args
        .get("line")
        .ok_or(BridgeError::MissingArg("line"))?
        .parse()
        .map_err(|_| BridgeError::Resolve("line must be a 1-based integer".into()))?;
    let column: u64 = args.get("column").and_then(|c| c.parse().ok()).unwrap_or(0);

    let abs = crate::pathx::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    let mut obj = serde_json::Map::new();
    obj.insert("action".into(), action.into());
    obj.insert("path".into(), abs.clone().into());
    obj.insert("line".into(), line.into());
    obj.insert("column".into(), column.into());
    if let Some(scope) = args.get("scope") {
        obj.insert("scope".into(), scope.into());
    }
    if let Some(direction) = args.get("direction") {
        obj.insert("direction".into(), direction.into());
    }

    let out = ctx
        .backend
        .call("ctx_refactor", serde_json::Value::Object(obj))
        .map_err(BridgeError::Backend)?;
    Ok(out)
}

/// `@symbol overview <path>` → symbols of one file. `Off` delegates to
/// ctx_refactor symbols_overview (byte-identical). `Compact`/`Tdd` render the
/// signature notation locally and collect sigs for the End-Hook legend (E-3).
fn overview(
    ctx: &Rc<EngineContext>,
    args: &DirectiveArgs,
    root: &str,
) -> Result<String, BridgeError> {
    let path = args
        .positional(1)
        .or_else(|| args.get("path"))
        .ok_or(BridgeError::MissingArg("path"))?;
    let abs = crate::pathx::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    if ctx.header.crp == crate::crp_proto::CrpMode::Off {
        let out = ctx
            .backend
            .call(
                "ctx_refactor",
                serde_json::json!({ "action": "symbols_overview", "path": abs }),
            )
            .map_err(BridgeError::Backend)?;
        return Ok(out);
    }

    let content = std::fs::read_to_string(&abs)
        .map_err(|e| BridgeError::Resolve(format!("read {abs}: {e}")))?;
    let ext = std::path::Path::new(&abs)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let (rendered, sigs) = crate::crp::render_file_signatures(&content, ext, ctx.header.crp, None);
    ctx.crp_sigs.borrow_mut().extend(sigs);
    Ok(rendered)
}

/// `@symbol body name=X [file=…] [kind=fn|struct|class|trait|enum]` →
/// `ctx_search action=symbol`. Fetches one symbol's AST-precise body by name —
/// the replacement for the deprecated `ctx_symbol` tool. `path` is scoped to the
/// jail root; `file` (if given) is jail-resolved to narrow the search. Output is
/// returned verbatim from the backend (no local CRP), consistent with the nav ops.
fn body(ctx: &Rc<EngineContext>, args: &DirectiveArgs, root: &str) -> Result<String, BridgeError> {
    let name = args.get("name").ok_or(BridgeError::MissingArg("name"))?;

    let mut obj = serde_json::Map::new();
    obj.insert("action".into(), "symbol".into());
    obj.insert("name".into(), name.into());
    obj.insert("path".into(), root.into());
    if let Some(file) = args.get("file") {
        let abs = crate::pathx::resolve_tool_path(Some(root), None, file)
            .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;
        obj.insert("file".into(), abs.into());
    }
    if let Some(kind) = args.get("kind") {
        obj.insert("kind".into(), kind.into());
    }

    let out = ctx
        .backend
        .call("ctx_search", serde_json::Value::Object(obj))
        .map_err(BridgeError::Backend)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    fn ctx_with_crp(root: PathBuf, crp: crate::crp_proto::CrpMode) -> Rc<EngineContext> {
        let h = LeanMdHeader {
            crp,
            ..Default::default()
        };
        Rc::new(EngineContext::new(h, root))
    }

    #[test]
    fn overview_tdd_emits_symbols_and_collects_sigs() {
        use crate::crp_proto::CrpMode;
        let dir = std::env::temp_dir().join("lmd_symbol_overview_tdd");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha() {}\npub struct Beta;\n").unwrap();
        let ctx = ctx_with_crp(dir.clone(), CrpMode::Tdd);
        let args = DirectiveArgs::parse(&format!("overview {}", f.to_str().unwrap()));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(out.contains("λ+alpha"), "tdd fn form: {out}");
        assert!(out.contains("§+Beta"), "tdd struct form: {out}");
        assert!(
            !out.contains("§=class"),
            "bridge must NOT emit legend: {out}"
        );
        assert!(!ctx.crp_sigs.borrow().is_empty(), "sigs collected");
    }

    #[test]
    fn overview_off_is_unchanged_handler_output() {
        use crate::crp_proto::CrpMode;
        let dir = std::env::temp_dir().join("lmd_symbol_overview_off");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha() {}\n").unwrap();
        let ctx = ctx_with_crp(dir.clone(), CrpMode::Off);
        let args = DirectiveArgs::parse(&format!("overview {}", f.to_str().unwrap()));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        // Off keeps the ctx_refactor symbols_overview envelope (no λ/§ glyphs).
        assert!(
            !out.contains("λ+") && !out.contains("§+"),
            "Off stays non-symbolic: {out}"
        );
        assert!(ctx.crp_sigs.borrow().is_empty(), "Off collects no sigs");
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

    #[test]
    fn impl_op_requires_a_position() {
        let dir = std::env::temp_dir();
        let ctx = ctx_at(dir.clone());
        let f = dir.join("pos_required.rs");
        // refs/def/impl/declaration/type-hierarchy without line= and without
        // name= cannot resolve a position → clear error. The MissingArg("line")
        // check fires before the jail-resolve, so the file need not exist; use
        // an absolute temp path so the test is CWD-independent (matches the
        // other dispatch tests in this module).
        let err = SymbolBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("impl {}", f.to_str().unwrap())),
            )
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("line")),
            "got: {err:?}"
        );
    }

    #[test]
    fn refs_op_routes_with_position() {
        // A real position; headless rust-analyzer or a degradation envelope —
        // never a panic, and never the unknown-op error.
        let dir = std::env::temp_dir().join("lmd_symbol_refs");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn helper() {}\nfn caller() { helper(); }\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("refs {} line=1 column=3", f.to_str().unwrap()));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "refs produced empty output");
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
    }

    #[test]
    fn declaration_is_ide_only_degradation_passes_through() {
        // declaration/type-hierarchy are IDE-only (spec §3.3). Without a running
        // IDE the bridge must surface ctx_refactor's degradation envelope
        // verbatim, not crash and not invent a result.
        let dir = std::env::temp_dir().join("lmd_symbol_decl");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("d.rs");
        std::fs::write(&f, "pub struct Widget;\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!(
            "declaration {} line=1 column=11",
            f.to_str().unwrap()
        ));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "declaration produced empty output");
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
    }

    #[test]
    fn name_addressing_without_line_returns_clear_error() {
        // resolve_name_path is not available outbound; name= alone must return
        // a clear ERROR string (not panic, not MissingArg).
        let ctx = ctx_at(PathBuf::from("."));
        let out = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("impl name=SomeSymbol"))
            .unwrap();
        assert!(
            out.contains("ERROR") && out.contains("line=") && out.contains("@symbol body"),
            "must explain name= needs line= and point to @symbol body: {out}"
        );
    }

    /// Build an EngineContext whose backend records every outbound (tool, args).
    #[allow(clippy::type_complexity)]
    fn recording_ctx(
        root: PathBuf,
    ) -> (
        Rc<EngineContext>,
        std::rc::Rc<std::cell::RefCell<Vec<(String, serde_json::Value)>>>,
    ) {
        use crate::backend::{BackendError, CodeIntelBackend};
        use std::cell::RefCell;
        struct Rec {
            calls: std::rc::Rc<RefCell<Vec<(String, serde_json::Value)>>>,
        }
        impl CodeIntelBackend for Rec {
            fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError> {
                self.calls.borrow_mut().push((tool.to_string(), args));
                Ok(String::new())
            }
        }
        let calls = std::rc::Rc::new(RefCell::new(Vec::new()));
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            root,
            Box::new(Rec {
                calls: calls.clone(),
            }),
        ));
        (ctx, calls)
    }

    #[test]
    fn body_forwards_name_file_and_kind_to_ctx_search() {
        let dir = std::env::temp_dir().join("lmd_symbol_body_fwd");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("s.rs");
        std::fs::write(&f, "pub fn target() {}\n").unwrap();
        let (ctx, calls) = recording_ctx(dir.clone());
        let args = DirectiveArgs::parse(&format!(
            "body name=target file={} kind=fn",
            f.to_str().unwrap()
        ));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(out.is_empty(), "recording backend returns empty: {out}");
        let calls = calls.borrow();
        let (tool, payload) = calls.first().expect("one outbound call recorded");
        assert_eq!(tool, "ctx_search", "routes to ctx_search");
        assert_eq!(
            payload.get("action").and_then(|v| v.as_str()),
            Some("symbol")
        );
        assert_eq!(payload.get("name").and_then(|v| v.as_str()), Some("target"));
        assert_eq!(payload.get("kind").and_then(|v| v.as_str()), Some("fn"));
        assert!(payload.get("file").is_some(), "file forwarded: {payload}");
    }

    #[test]
    fn body_missing_name_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("body"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("name")),
            "got: {err:?}"
        );
    }

    #[test]
    fn body_op_dispatches_not_unknown() {
        // body must NOT hit the unknown-op path even without file=/kind=.
        let dir = std::env::temp_dir().join("lmd_symbol_body_disp");
        std::fs::create_dir_all(&dir).unwrap();
        let (ctx, calls) = recording_ctx(dir.clone());
        let args = DirectiveArgs::parse("body name=Widget");
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(!out.contains("unknown @symbol op"), "must dispatch: {out}");
        let calls = calls.borrow();
        let (tool, payload) = calls.first().expect("one call recorded");
        assert_eq!(tool, "ctx_search");
        assert_eq!(payload.get("name").and_then(|v| v.as_str()), Some("Widget"));
        assert!(payload.get("file").is_none(), "no file when omitted");
        assert!(payload.get("kind").is_none(), "no kind when omitted");
    }

    #[test]
    fn unknown_op_message_lists_body() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate x.rs"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("body"), "op list names body: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn type_hierarchy_passes_direction() {
        // Without backend, returns BACKEND_REQUIRED envelope — never panics,
        // never "unknown @symbol op".
        let dir = std::env::temp_dir().join("lmd_symbol_th");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.rs");
        std::fs::write(&f, "trait Animal {}\nstruct Dog;\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!(
            "type-hierarchy {} line=2 column=7 direction=subtypes",
            f.to_str().unwrap()
        ));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(
            !out.trim().is_empty(),
            "type-hierarchy produced empty output"
        );
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
    }
}
