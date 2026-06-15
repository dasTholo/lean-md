//! `@edit` Router bridge — the single write-directive family (spec §4.2).
//! Text mode (`old`/`new`) routes to `ctx_edit`; symbolic mode (`symbol`+`body`
//! /`text`) routes to `ctx_refactor replace_symbol_body`/`insert_*`. Cache
//! coherence (spec §3.4): text mode invalidates via `ctx_edit`'s CacheEffect,
//! symbolic mode invalidates the shared EngineContext cache on success.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

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
    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    let params = crate::tools::ctx_edit::EditParams {
        path: abs,
        old_string: old.to_string(),
        new_string: new.to_string(),
        replace_all: args.get("all") == Some("true"),
        create: args.get("create") == Some("true"),
        expected_md5: None,
        expected_size: None,
        expected_mtime_ms: None,
        backup: false,
        backup_path: None,
        evidence: true,
        diff_max_lines: 200,
        allow_lossy_utf8: false,
    };

    // Shared session cache — ctx_edit applies CacheEffect::Invalidate on success,
    // so the next `@read` of this path re-validates by mtime (spec §3.4). NOT a
    // per-call SessionCache::new().
    let mut cache = ctx.cache.borrow_mut();
    Ok(crate::tools::ctx_edit::handle(&mut cache, &params))
}

fn symbolic_edit(_ctx: &Rc<EngineContext>, _args: &DirectiveArgs) -> Result<String, BridgeError> {
    // Implemented in Task 3.
    Err(BridgeError::Resolve("symbolic @edit not yet implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::header::LeanMdHeader;

    fn ctx_at(root: &std::path::Path) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            root.to_path_buf(),
        ))
    }

    #[test]
    fn text_edit_replaces_and_invalidates_cache() {
        let dir = std::env::temp_dir().join("lmd_edit_text");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.txt");
        std::fs::write(&f, "alpha BEFORE omega\n").unwrap();
        let ctx = ctx_at(&dir);

        // Warm the cache with the OLD content first (full read).
        {
            let mut cache = ctx.cache.borrow_mut();
            let _ = crate::tools::ctx_read::handle(
                &mut cache,
                f.to_str().unwrap(),
                "full",
                crate::tools::CrpMode::Off,
            );
        }

        let args = DirectiveArgs::parse(&format!(
            r#"{} old="BEFORE" new="AFTER""#,
            f.to_str().unwrap()
        ));
        let out = EditBridge.execute(&ctx, &args).unwrap();
        assert!(!out.starts_with("ERROR"), "edit must succeed, got: {out}");
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "alpha AFTER omega\n");

        // Post-edit read must show NEW bytes — proves the cache was invalidated.
        let reread = {
            let mut cache = ctx.cache.borrow_mut();
            crate::tools::ctx_read::handle(
                &mut cache,
                f.to_str().unwrap(),
                "full",
                crate::tools::CrpMode::Off,
            )
        };
        assert!(reread.contains("AFTER"), "stale cache hit, got: {reread}");
        assert!(!reread.contains("BEFORE"), "must not show old bytes: {reread}");
    }

    #[test]
    fn missing_old_errors() {
        let dir = std::env::temp_dir();
        let ctx = ctx_at(&dir);
        let err = EditBridge
            .execute(&ctx, &DirectiveArgs::parse("some.txt"))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("old")), "got: {err:?}");
    }
}
