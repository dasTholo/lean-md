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
        let _op_stem = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @refactor op '{op}'. Use: rename|move|safe-delete|inline"
            ))
        })?;

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let (_obj, _abs) = build_target(args, root)?;

        // Backend dispatch (preview + apply) is implemented in Tasks 2/3.
        // For now surface a clear placeholder so callers know the op was
        // recognised but the action is not yet wired.
        Ok(format!(
            "PENDING: @refactor {op} — backend dispatch not yet implemented (Task 2/3)"
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
}
