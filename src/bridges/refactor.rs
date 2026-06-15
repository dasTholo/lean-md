//! `@refactor` Router bridge → two-phase structural refactoring via the
//! lean-ctx `ctx_refactor` IDE/LSP backend (spec §4.2).
//! Four ops: rename / move / safe-delete / inline.
//! Task 1 (Phase 3.3): skeleton + op-mapping + addressing. Backend dispatch
//! (preview + apply phases) lands in Tasks 2/3.

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

impl DirectiveBridge for RefactorBridge {
    fn name(&self) -> &'static str {
        "refactor"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
        let op_stem = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @refactor op '{op}'. Use: rename|move|safe-delete|inline"
            ))
        })?;

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let (mut obj, abs) = build_target(args, root)?;

        // Positional-flag detection (same pattern as edit.rs symbolic_edit).
        let flag = |w: &str| {
            (1_usize..)
                .map_while(|i| args.positional(i))
                .any(|t| t == w)
        };

        // Task 2: preview path only.
        // TODO(Task 3): replace with apply branch keyed on args.get("plan_hash") —
        // when plan_hash= is present use "_apply", otherwise "_preview".
        let action_suffix = "_preview";

        let action = format!("{op_stem}{action_suffix}");
        obj.insert("action".into(), action.into());

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
            }
            "safe_delete" => {
                // No mandatory extras for safe-delete.
            }
            "inline" => {
                if flag("keep-definition") {
                    obj.insert("keep_definition".into(), true.into());
                }
            }
            _ => {}
        }

        // Preview is read-only — NO cache clear (apply-path cache clear is Task 3).
        let out = crate::tools::ctx_refactor::handle(&serde_json::Value::Object(obj), root, &abs);
        Ok(out)
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
}
