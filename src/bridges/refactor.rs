//! `@refactor` Router bridge → two-phase structural refactoring via the
//! lean-ctx `ctx_refactor` IDE/LSP backend (spec §4.2).
//! Four ops: rename / move / safe-delete / inline.
//! Task 1 (Phase 3.3): skeleton + op-mapping + addressing.
//! Task 2 (Phase 3.3): preview-path backend dispatch.
//! Task 3 (Phase 3.3): apply-path (plan_hash= → *_apply) + cache coherence.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct RefactorBridge;

/// Map a user-facing op token to the ctx_refactor action stem.
/// `safe-delete` is the user-facing spelling; the stem `safe_delete` is used
/// to construct `safe_delete_preview` / `safe_delete_apply` in Tasks 2/3.
/// Returns `None` for unknown ops.
fn map_op(op: &str) -> Option<&'static str> {
    Some(match op {
        "rename" => "rename",
        "move" => "move",
        "safe-delete" => "safe_delete",
        "inline" => "inline",
        _ => return None,
    })
}

/// Shared target-address map builder.  Called once per `execute` so Tasks 2/3
/// can reuse it without duplicating the addressing logic.
///
/// Returns `(obj, abs_path)` where:
/// - `obj` is a `serde_json::Map` pre-populated with either
///   `{ "name_path": … }` (name= addressing) or
///   `{ "line": …, "column": … }` (path= addressing); and
/// - `abs_path` is the jail-resolved absolute path string
///   (empty string when `name=` addressing is used — ctx_refactor resolves
///   name_path via its own symbol index, so no path is needed here).
pub(crate) fn build_target(
    args: &DirectiveArgs,
    root: &str,
) -> Result<(serde_json::Map<String, serde_json::Value>, String), BridgeError> {
    let mut obj = serde_json::Map::new();

    if let Some(name) = args.get("name") {
        // name= addressing: ctx_refactor resolves the symbol via its index.
        obj.insert("name_path".into(), name.into());
        return Ok((obj, String::new()));
    }

    // path= + line= + column= addressing (positional path is NOT used for
    // @refactor — plan spec: `path=` named arg only).
    let path = args.get("path").ok_or(BridgeError::MissingArg("path"))?;
    let line: u64 = args
        .get("line")
        .ok_or(BridgeError::MissingArg("line"))?
        .parse()
        .map_err(|_| BridgeError::Resolve("line must be a 1-based integer".into()))?;
    let column: u64 = args.get("column").and_then(|c| c.parse().ok()).unwrap_or(0);

    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    obj.insert("line".into(), line.into());
    obj.insert("column".into(), column.into());
    Ok((obj, abs))
}

/// Build the full action string and op-specific dispatch map.
///
/// Returns `(action, obj, abs_path)` where:
/// - `action` is the full `ctx_refactor` action name
///   (e.g. `"rename_preview"`, `"rename_apply"`),
/// - `obj` is the populated params map ready for `ctx_refactor::handle`, and
/// - `abs_path` is the jail-resolved absolute path (empty for name= addressing).
///
/// Extracted from `execute` so unit tests can assert map contents without
/// triggering a live backend call (real apply is IDE-only).
pub(crate) fn build_action_and_map(
    args: &DirectiveArgs,
    root: &str,
) -> Result<(String, serde_json::Map<String, serde_json::Value>, String), BridgeError> {
    let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
    let op_stem = map_op(op).ok_or_else(|| {
        BridgeError::Resolve(format!(
            "unknown @refactor op '{op}'. Use: rename|move|safe-delete|inline"
        ))
    })?;

    let (mut obj, abs) = build_target(args, root)?;

    // Positional-flag detection (same pattern as edit.rs symbolic_edit).
    let flag = |w: &str| {
        (1_usize..)
            .map_while(|i| args.positional(i))
            .any(|t| t == w)
    };

    // Phase switch: plan_hash= presence selects _apply, absence → _preview.
    // plan_hash value is passed verbatim — backend requires it to guard stale plans.
    let action_suffix = if let Some(h) = args.get("plan_hash") {
        obj.insert("plan_hash".into(), h.into());
        "_apply"
    } else {
        "_preview"
    };

    let action = format!("{op_stem}{action_suffix}");
    obj.insert("action".into(), action.clone().into());

    // Op-specific flag/arg mapping.
    match op_stem {
        "rename" => {
            let new_name = args.get("new").ok_or(BridgeError::MissingArg("new"))?;
            obj.insert("new_name".into(), new_name.into());
            if flag("search-comments") {
                obj.insert("search_comments".into(), true.into());
            }
            if flag("search-text") {
                obj.insert("search_text_occurrences".into(), true.into());
            }
            // force supported for rename.
            if flag("force") {
                obj.insert("force".into(), true.into());
            }
        }
        "move" => {
            let target = args.get("target");
            let parent = args.get("parent");
            match (target, parent) {
                (None, None) => return Err(BridgeError::MissingArg("target")),
                (Some(t), Some(p)) => {
                    // Pass BOTH keys through so ctx_refactor::resolve_move_target
                    // fires INVALID_TARGET (spec: XOR enforced by backend, not bridge).
                    obj.insert("target_path".into(), t.into());
                    obj.insert("target_parent".into(), p.into());
                }
                (Some(t), None) => {
                    obj.insert("target_path".into(), t.into());
                }
                (None, Some(p)) => {
                    obj.insert("target_parent".into(), p.into());
                }
            }
            // force supported for move.
            if flag("force") {
                obj.insert("force".into(), true.into());
            }
        }
        "safe_delete" => {
            // force supported for safe-delete.
            if flag("force") {
                obj.insert("force".into(), true.into());
            }
            // propagate is only meaningful for safe-delete.
            if flag("propagate") {
                obj.insert("propagate".into(), true.into());
            }
        }
        "inline" => {
            if flag("keep-definition") {
                obj.insert("keep_definition".into(), true.into());
            }
            // NOTE: `force` is intentionally NOT mapped for inline.
            // inline is not forceable — the backend returns UNSUPPORTED.
            // Silently dropping it here prevents a confusing backend error.
        }
        _ => {}
    }

    Ok((action, obj, abs))
}

impl DirectiveBridge for RefactorBridge {
    fn name(&self) -> &'static str {
        "refactor"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let root = ctx.jail_root.to_str().unwrap_or(".");
        let (action, obj, abs) = build_action_and_map(args, root)?;

        let out = crate::tools::ctx_refactor::handle(&serde_json::Value::Object(obj), root, &abs);

        // Cache coherence (spec §3.4): mirror edit.rs:symbolic_edit exactly.
        // Apply-path only — preview is read-only and must NEVER clear the cache.
        // CONFLICT arrives as "ERROR: CONFLICT: …" — already covered by the ERROR prefix.
        if apply_succeeded(&action, &out) {
            ctx.cache.borrow_mut().clear();
        }

        Ok(out)
    }
}

/// Apply succeeded and the shared cache must be invalidated (spec §3.4):
/// only on the apply phase, and only when the backend reports neither an
/// ERROR envelope nor a "not applied" no-op. Mirrors edit.rs:symbolic_edit.
pub(crate) fn apply_succeeded(action: &str, out: &str) -> bool {
    action.ends_with("_apply") && !out.starts_with("ERROR") && !out.contains("not applied")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    // ── op-mapping ────────────────────────────────────────────────────────────

    #[test]
    fn op_stems_map_correctly() {
        assert_eq!(map_op("rename"), Some("rename"));
        assert_eq!(map_op("move"), Some("move"));
        assert_eq!(map_op("safe-delete"), Some("safe_delete"));
        assert_eq!(map_op("inline"), Some("inline"));
        assert_eq!(map_op("frobnicate"), None);
        assert_eq!(map_op(""), None);
    }

    // ── missing / unknown op ──────────────────────────────────────────────────

    #[test]
    fn missing_op_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("op")), "got: {err:?}");
    }

    #[test]
    fn unknown_op_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate name=Foo"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => {
                assert!(m.contains("unknown @refactor op"), "got: {m}")
            }
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    // ── address-map builder ────────────────────────────────────────────────────

    #[test]
    fn name_addressing_inserts_name_path() {
        let (obj, abs) = build_target(&DirectiveArgs::parse("rename name=MyStruct"), ".").unwrap();
        assert_eq!(
            obj.get("name_path").and_then(|v| v.as_str()),
            Some("MyStruct")
        );
        assert!(obj.get("line").is_none(), "no line when name= used");
        assert!(obj.get("column").is_none(), "no column when name= used");
        assert!(
            abs.is_empty(),
            "abs_path must be empty for name= addressing"
        );
    }

    #[test]
    fn path_addressing_inserts_line_and_column() {
        let dir = std::env::temp_dir().join("lmd_refactor_addr");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();

        let input = format!("rename path={} line=1 column=3", f.to_str().unwrap());
        let (obj, abs) =
            build_target(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();

        assert_eq!(obj.get("line").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(obj.get("column").and_then(|v| v.as_u64()), Some(3));
        assert!(obj.get("name_path").is_none());
        assert!(!abs.is_empty(), "abs_path must be set for path= addressing");
    }

    #[test]
    fn path_addressing_column_defaults_to_zero() {
        let dir = std::env::temp_dir().join("lmd_refactor_coldef");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("b.rs");
        std::fs::write(&f, "fn bar() {}\n").unwrap();

        let input = format!("rename path={} line=1", f.to_str().unwrap());
        let (obj, _) = build_target(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();

        assert_eq!(obj.get("column").and_then(|v| v.as_u64()), Some(0));
    }

    #[test]
    fn path_addressing_missing_line_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let dir = std::env::temp_dir().join("lmd_refactor_noline");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("c.rs");
        std::fs::write(&f, "fn baz() {}\n").unwrap();

        let err = RefactorBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("rename path={}", f.to_str().unwrap())),
            )
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("line")),
            "got: {err:?}"
        );
    }

    #[test]
    fn missing_path_and_name_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse("rename"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("path")),
            "got: {err:?}"
        );
    }

    // ── registry ──────────────────────────────────────────────────────────────

    #[test]
    fn refactor_is_registered() {
        assert!(super::super::default_registry().get("refactor").is_some());
    }

    // ── Task 2: preview-path dispatch ────────────────────────────────────────

    /// Helper: ctx rooted at a real temp dir with a Rust source file.
    fn ctx_with_file(dir_suffix: &str) -> (Rc<EngineContext>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(dir_suffix);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("subject.rs");
        std::fs::write(&f, "pub fn my_func() {}\n").unwrap();
        (ctx_at(dir.clone()), f)
    }

    /// All four preview ops must return a clean BACKEND_REQUIRED envelope (or
    /// ERROR envelope) — never a panic, never MissingArg — when no IDE is
    /// present (headless).
    #[test]
    fn rename_preview_returns_backend_required_envelope() {
        let (ctx, f) = ctx_with_file("lmd_refactor_rename_preview");
        let input = format!(
            "rename path={} line=1 new=renamed_func",
            f.to_str().unwrap()
        );
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "rename preview must degrade cleanly, got: {out}"
        );
    }

    #[test]
    fn move_preview_returns_backend_required_envelope() {
        let (ctx, f) = ctx_with_file("lmd_refactor_move_preview");
        let input = format!("move path={} line=1 target=other/", f.to_str().unwrap());
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "move preview must degrade cleanly, got: {out}"
        );
    }

    #[test]
    fn safe_delete_preview_returns_backend_required_envelope() {
        let (ctx, f) = ctx_with_file("lmd_refactor_safedel_preview");
        let input = format!("safe-delete path={} line=1", f.to_str().unwrap());
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "safe-delete preview must degrade cleanly, got: {out}"
        );
    }

    #[test]
    fn inline_preview_returns_backend_required_envelope() {
        let (ctx, f) = ctx_with_file("lmd_refactor_inline_preview");
        let input = format!("inline path={} line=1", f.to_str().unwrap());
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "inline preview must degrade cleanly, got: {out}"
        );
    }

    /// move without target= or parent= → MissingArg("target").
    #[test]
    fn move_without_target_errors() {
        let (ctx, f) = ctx_with_file("lmd_refactor_move_notarget");
        let err = RefactorBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("move path={} line=1", f.to_str().unwrap())),
            )
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("target")),
            "got: {err:?}"
        );
    }

    /// rename without new= → MissingArg("new").
    #[test]
    fn rename_without_new_errors() {
        let (ctx, f) = ctx_with_file("lmd_refactor_rename_nonew");
        let err = RefactorBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("rename path={} line=1", f.to_str().unwrap())),
            )
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("new")),
            "got: {err:?}"
        );
    }

    /// move with parent= (no target=) → not a MissingArg, reaches backend.
    #[test]
    fn move_with_parent_reaches_backend() {
        let (ctx, f) = ctx_with_file("lmd_refactor_move_parent");
        let input = format!(
            "move path={} line=1 parent=OtherStruct",
            f.to_str().unwrap()
        );
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "move(parent=) must degrade cleanly, got: {out}"
        );
    }

    /// move with BOTH target= and parent= → ctx_refactor::resolve_move_target fires
    /// INVALID_TARGET (XOR is enforced by the backend, not the bridge).
    #[test]
    fn move_both_target_and_parent_returns_invalid_target() {
        let (ctx, f) = ctx_with_file("lmd_refactor_move_both");
        let input = format!(
            "move path={} line=1 target=other/ parent=OtherStruct",
            f.to_str().unwrap()
        );
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("INVALID_TARGET"),
            "both target+parent must yield INVALID_TARGET, got: {out}"
        );
    }

    /// rename with search-comments / search-text flags → clean backend call.
    #[test]
    fn rename_preview_with_flags_reaches_backend() {
        let (ctx, f) = ctx_with_file("lmd_refactor_rename_flags");
        let input = format!(
            "rename search-comments search-text path={} line=1 new=bar",
            f.to_str().unwrap()
        );
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "rename(flags) must degrade cleanly, got: {out}"
        );
    }

    /// inline with keep-definition flag → clean backend call.
    #[test]
    fn inline_preview_keep_definition_reaches_backend() {
        let (ctx, f) = ctx_with_file("lmd_refactor_inline_keepdef");
        let input = format!("inline keep-definition path={} line=1", f.to_str().unwrap());
        let out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "inline(keep-definition) must degrade cleanly, got: {out}"
        );
    }

    // ── Task 3: apply-path, force/propagate mapping, cache coherence ─────────

    /// plan_hash= selects _apply action; value is passed verbatim.
    #[test]
    fn plan_hash_selects_apply_action() {
        let dir = std::env::temp_dir().join("lmd_refactor_apply_action");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn renamed() {}\n").unwrap();
        let input = format!(
            "rename path={} line=1 new=other plan_hash=abc123",
            f.to_str().unwrap()
        );
        let (action, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(action, "rename_apply", "action must be rename_apply");
        assert_eq!(
            obj.get("plan_hash").and_then(|v| v.as_str()),
            Some("abc123"),
            "plan_hash must be passed verbatim"
        );
    }

    /// Without plan_hash= the action is _preview (coupling test).
    #[test]
    fn no_plan_hash_selects_preview_action() {
        let dir = std::env::temp_dir().join("lmd_refactor_preview_action");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!("rename path={} line=1 new=bar", f.to_str().unwrap());
        let (action, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(action, "rename_preview");
        assert!(
            obj.get("plan_hash").is_none(),
            "plan_hash must not be present for preview"
        );
    }

    /// `force` is mapped for rename when plan_hash= is set.
    #[test]
    fn rename_apply_with_force_sets_force_key() {
        let dir = std::env::temp_dir().join("lmd_refactor_rename_force");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!(
            "rename force path={} line=1 new=bar plan_hash=H",
            f.to_str().unwrap()
        );
        let (action, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(action, "rename_apply");
        assert_eq!(
            obj.get("force").and_then(|v| v.as_bool()),
            Some(true),
            "rename apply with force flag must set force=true"
        );
    }

    /// `force` is mapped for move.
    #[test]
    fn move_apply_with_force_sets_force_key() {
        let dir = std::env::temp_dir().join("lmd_refactor_move_force");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!(
            "move force path={} line=1 target=other/ plan_hash=H",
            f.to_str().unwrap()
        );
        let (action, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(action, "move_apply");
        assert_eq!(obj.get("force").and_then(|v| v.as_bool()), Some(true));
    }

    /// `force` is mapped for safe-delete.
    #[test]
    fn safe_delete_apply_with_force_sets_force_key() {
        let dir = std::env::temp_dir().join("lmd_refactor_safedel_force");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!(
            "safe-delete force path={} line=1 plan_hash=H",
            f.to_str().unwrap()
        );
        let (action, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(action, "safe_delete_apply");
        assert_eq!(obj.get("force").and_then(|v| v.as_bool()), Some(true));
    }

    /// `propagate` is only mapped for safe-delete.
    #[test]
    fn safe_delete_apply_with_propagate_sets_propagate_key() {
        let dir = std::env::temp_dir().join("lmd_refactor_safedel_prop");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!(
            "safe-delete propagate path={} line=1 plan_hash=H",
            f.to_str().unwrap()
        );
        let (_, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(obj.get("propagate").and_then(|v| v.as_bool()), Some(true));
    }

    /// CRITICAL: `force` is NOT mapped for inline — even with plan_hash= + force flag.
    /// inline is not forceable; the backend returns UNSUPPORTED if force is sent.
    #[test]
    fn inline_apply_force_does_not_set_force_key() {
        let dir = std::env::temp_dir().join("lmd_refactor_inline_noforce");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!(
            "inline force path={} line=1 plan_hash=H",
            f.to_str().unwrap()
        );
        let (action, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert_eq!(action, "inline_apply");
        assert!(
            obj.get("force").is_none(),
            "inline must NOT receive force key, got: {:?}",
            obj.get("force")
        );
    }

    /// `propagate` is NOT mapped for inline (only safe-delete).
    #[test]
    fn inline_apply_propagate_not_mapped() {
        let dir = std::env::temp_dir().join("lmd_refactor_inline_noprop");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();
        let input = format!(
            "inline propagate path={} line=1 plan_hash=H",
            f.to_str().unwrap()
        );
        let (_, obj, _) =
            build_action_and_map(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();
        assert!(
            obj.get("propagate").is_none(),
            "inline must NOT receive propagate key"
        );
    }

    /// Cache-clear branch: a synthetic success string (does not start with "ERROR",
    /// does not contain "not applied") on the apply path clears the cache.
    /// Real apply is IDE-only; this test exercises the production predicate directly.
    #[test]
    fn apply_success_predicate_clears_cache() {
        // Drive the production predicate — not an inline copy.
        // If apply_succeeded drifts this test immediately fails.
        let cases: &[(&str, &str, bool)] = &[
            ("rename_apply", "Applied successfully", true), // clear
            ("rename_apply", "ERROR: CONFLICT: …", false),  // no clear (error)
            ("rename_apply", "Changes not applied", false), // no clear (not applied)
            ("rename_preview", "Applied successfully", false), // no clear (preview path)
            ("rename_apply", "ERROR: BACKEND_REQUIRED: …", false), // no clear (error)
        ];
        for (action, out, expect_clear) in cases {
            let should_clear = super::apply_succeeded(action, out);
            assert_eq!(
                should_clear, *expect_clear,
                "action={action:?} out={out:?}: expected clear={expect_clear}, got {should_clear}"
            );
        }
    }

    /// Integration: a preview call must NOT clear the cache even if the backend
    /// returns a non-error string.
    #[test]
    fn preview_never_clears_cache() {
        let dir = std::env::temp_dir().join("lmd_refactor_preview_noclear");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn my_func() {}\n").unwrap();
        let ctx = Rc::new(crate::lmd::engine::EngineContext::new(
            LeanMdHeader::default(),
            dir.clone(),
        ));

        // Warm the cache via store() so we can detect if it's cleared.
        let path_str = f.to_str().unwrap();
        ctx.cache.borrow_mut().store(path_str, "cached-content");

        let input = format!("rename path={} line=1 new=other", path_str);
        let _out = RefactorBridge
            .execute(&ctx, &DirectiveArgs::parse(&input))
            .unwrap();

        // Preview must never clear — the stored entry must still be present.
        assert!(
            ctx.cache.borrow().get(path_str).is_some(),
            "preview must NOT clear the cache"
        );
    }

    /// plan_hash coupling: the only way to reach *_apply via this bridge is by
    /// providing plan_hash=. Without it the bridge structurally selects _preview,
    /// making `*_apply` without a plan_hash unreachable through normal use.
    /// This test documents and asserts that structural invariant.
    #[test]
    fn apply_path_requires_plan_hash_coupling() {
        let dir = std::env::temp_dir().join("lmd_refactor_coupling");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("sub.rs");
        std::fs::write(&f, "pub fn foo() {}\n").unwrap();

        // Without plan_hash → always _preview, regardless of intent.
        let (action_no_hash, _, _) = build_action_and_map(
            &DirectiveArgs::parse(&format!(
                "rename path={} line=1 new=bar",
                f.to_str().unwrap()
            )),
            dir.to_str().unwrap(),
        )
        .unwrap();
        assert!(
            action_no_hash.ends_with("_preview"),
            "no plan_hash → must be _preview"
        );

        // With plan_hash → _apply.
        let (action_with_hash, _, _) = build_action_and_map(
            &DirectiveArgs::parse(&format!(
                "rename path={} line=1 new=bar plan_hash=abc",
                f.to_str().unwrap()
            )),
            dir.to_str().unwrap(),
        )
        .unwrap();
        assert!(
            action_with_hash.ends_with("_apply"),
            "plan_hash → must be _apply"
        );
    }
}
