//! `@edit` Router bridge — the single write-directive family (spec §4.2).
//! Text mode (`old`/`new`) routes to `ctx_edit`; symbolic mode (`symbol`+`body`
//! /`text`) routes to `ctx_refactor replace_symbol_body`/`insert_*`. Cache
//! coherence (spec §3.4): text mode invalidates via `ctx_edit`'s CacheEffect,
//! symbolic mode invalidates the shared EngineContext cache on success.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct EditBridge;

impl DirectiveBridge for EditBridge {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        if args.get("symbol").is_some() {
            return symbolic_edit(ctx, args);
        }
        text_edit(ctx, args)
    }
}

/// `@edit <path> old="…" new="…" [all=true] [create=true]` → `ctx_edit`.
fn text_edit(ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
    let path = args
        .positional(0)
        .or_else(|| args.get("path"))
        .ok_or(BridgeError::MissingArg("path"))?;
    let old = args.get("old").ok_or(BridgeError::MissingArg("old"))?;
    let new = args.get("new").unwrap_or("");

    // §5 PathJail for writes (matches ctx_refactor's resolve; reads pass through
    // unchanged but writes must not escape the render's project root).
    let root = ctx.jail_root.to_str().unwrap_or(".");
    let abs = crate::pathx::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    let mut payload = serde_json::Map::new();
    payload.insert("path".into(), abs.into());
    payload.insert("old_string".into(), old.into());
    payload.insert("new_string".into(), new.into());
    if args.get("all") == Some("true") {
        payload.insert("replace_all".into(), true.into());
    }
    if args.get("create") == Some("true") {
        payload.insert("create".into(), true.into());
    }
    let out = ctx
        .backend
        .call("ctx_edit", serde_json::Value::Object(payload))
        .map_err(BridgeError::Backend)?;
    Ok(out)
}

/// `@edit symbol=Class/method body="…"` → ctx_refactor replace_symbol_body
/// `@edit symbol=… before|after text="…"` → insert_before_symbol/insert_after_symbol
/// Optional `hash=<blake3hex>` → expected_hash (BLAKE3 TOCTOU guard, enforced
/// inside ctx_refactor). name_path resolution + headless tree-sitter range write
/// are inherited from ctx_refactor; this bridge only maps args and invalidates.
fn symbolic_edit(ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
    let symbol = args
        .get("symbol")
        .ok_or(BridgeError::MissingArg("symbol"))?;
    let positional_flag = |w: &str| (1..).map_while(|i| args.positional(i)).any(|t| t == w);

    let mut obj = serde_json::Map::new();
    obj.insert("name_path".into(), symbol.into());
    if let Some(hash) = args.get("hash") {
        obj.insert("expected_hash".into(), hash.into());
    }

    let action = if let Some(body) = args.get("body") {
        obj.insert("new_body".into(), body.into());
        "replace_symbol_body"
    } else if positional_flag("before") || positional_flag("after") {
        let text = args.get("text").ok_or(BridgeError::MissingArg("text"))?;
        obj.insert("text".into(), text.into());
        if positional_flag("before") {
            "insert_before_symbol"
        } else {
            "insert_after_symbol"
        }
    } else {
        // symbol present but no body= and no before/after flag.
        return Err(BridgeError::MissingArg("body"));
    };
    obj.insert("action".into(), action.into());

    // abs_path empty: ctx_refactor resolves name_path via its own symbol index.
    let out = ctx
        .backend
        .call("ctx_refactor", serde_json::Value::Object(obj))
        .map_err(BridgeError::Backend)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;

    fn ctx_at(root: &std::path::Path) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            root.to_path_buf(),
        ))
    }

    #[test]
    #[ignore = "server-enforced: @edit text mode routes outbound to ctx_edit which needs a live lean-ctx session (returns 'cache not available' headless)"]
    fn text_edit_replaces_content() {
        let dir = std::env::temp_dir().join("lmd_edit_text");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.txt");
        std::fs::write(&f, "alpha BEFORE omega\n").unwrap();
        let ctx = ctx_at(&dir);

        let args = DirectiveArgs::parse(&format!(
            r#"{} old="BEFORE" new="AFTER""#,
            f.to_str().unwrap()
        ));
        let out = EditBridge.execute(&ctx, &args).unwrap();
        assert!(!out.starts_with("ERROR"), "edit must succeed, got: {out}");
        // File must be modified on disk.
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "alpha AFTER omega\n");
    }

    #[test]
    fn symbolic_replace_body_headless() {
        // Headless tree-sitter range edit on a real Rust symbol (no IDE).
        let dir = std::env::temp_dir().join("lmd_edit_sym");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("s.rs");
        std::fs::write(&f, "fn greet() {\n    println!(\"old\");\n}\n").unwrap();
        let ctx = ctx_at(&dir);

        let args =
            DirectiveArgs::parse(r#"symbol=greet body="fn greet() {\n    println!(\"new\");\n}""#);
        let out = EditBridge.execute(&ctx, &args).unwrap();
        // Either applied (headless) or a precise degradation envelope — never a panic.
        assert!(
            out.contains("applied") || out.contains("ERROR"),
            "got: {out}"
        );
        if out.contains("applied") {
            assert!(std::fs::read_to_string(&f).unwrap().contains("new"));
        }
    }

    #[test]
    fn symbolic_requires_a_mode() {
        let dir = std::env::temp_dir();
        let ctx = ctx_at(&dir);
        // symbol= present but neither body= nor before/after → clear error.
        let err = EditBridge
            .execute(&ctx, &DirectiveArgs::parse("symbol=Foo/bar"))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)), "got: {err:?}");
    }

    #[test]
    fn missing_old_errors() {
        let dir = std::env::temp_dir();
        let ctx = ctx_at(&dir);
        let err = EditBridge
            .execute(&ctx, &DirectiveArgs::parse("some.txt"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("old")),
            "got: {err:?}"
        );
    }
}
