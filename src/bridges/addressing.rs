//! Shared symbol/position addressing for the code-intelligence bridges
//! (`@refactor`, `@reformat`). Extracted from `refactor.rs` (Phase 3.3) so the
//! `name_path | path[+line+column]` address-map logic lives in exactly one place.
//!
//! `symbol.rs` deliberately does NOT use this helper: its nav ops resolve a
//! `name=` reference *locally* to `path+line+column` (via `resolve_name_path` +
//! cache-backed `column_of`) and accept a positional path, whereas this helper
//! passes `name_path` *through* to the `ctx_refactor` backend. The two address
//! models are not interchangeable — see `symbol.rs::nav`.

use super::BridgeError;
use crate::lmd::args::DirectiveArgs;

/// Build the target-address map with `line=` REQUIRED for path addressing.
/// This is the `@refactor` contract (every position op needs a cursor line).
pub(crate) fn build_target(
    args: &DirectiveArgs,
    root: &str,
) -> Result<(serde_json::Map<String, serde_json::Value>, String), BridgeError> {
    build_target_with(args, root, true)
}

/// Build the target-address map.
///
/// Returns `(obj, abs_path)` where:
/// - `obj` is pre-populated with either `{ "name_path": … }` (name= addressing)
///   or `{ "line": …, "column": … }` (path= addressing, when a line is given); and
/// - `abs_path` is the jail-resolved absolute path string (empty for name=).
///
/// `require_line`:
/// - `true`  → path addressing without `line=` is an error (`@refactor`).
/// - `false` → path addressing without `line=` is valid whole-file/path-only
///   addressing; only `path` is resolved, no line/column keys are inserted
///   (`@reformat`: reformat-by-path).
pub(crate) fn build_target_with(
    args: &DirectiveArgs,
    root: &str,
    require_line: bool,
) -> Result<(serde_json::Map<String, serde_json::Value>, String), BridgeError> {
    let mut obj = serde_json::Map::new();

    if let Some(name) = args.get("name") {
        // name= addressing: ctx_refactor resolves the symbol via its index.
        obj.insert("name_path".into(), name.into());
        return Ok((obj, String::new()));
    }

    let path = args.get("path").ok_or(BridgeError::MissingArg("path"))?;
    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    match args.get("line") {
        Some(l) => {
            let line: u64 = l
                .parse()
                .map_err(|_| BridgeError::Resolve("line must be a 1-based integer".into()))?;
            let column: u64 = args.get("column").and_then(|c| c.parse().ok()).unwrap_or(0);
            obj.insert("line".into(), line.into());
            obj.insert("column".into(), column.into());
        }
        None if require_line => return Err(BridgeError::MissingArg("line")),
        None => { /* path-only: whole-file addressing, no line/column keys */ }
    }

    Ok((obj, abs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_addressing_inserts_name_path() {
        let (obj, abs) =
            build_target(&DirectiveArgs::parse("rename name=MyStruct"), ".").unwrap();
        assert_eq!(
            obj.get("name_path").and_then(|v| v.as_str()),
            Some("MyStruct")
        );
        assert!(obj.get("line").is_none(), "no line when name= used");
        assert!(obj.get("column").is_none(), "no column when name= used");
        assert!(abs.is_empty(), "abs_path must be empty for name= addressing");
    }

    #[test]
    fn path_addressing_inserts_line_and_column() {
        let dir = std::env::temp_dir().join("lmd_addr_lc");
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
        let dir = std::env::temp_dir().join("lmd_addr_coldef");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("b.rs");
        std::fs::write(&f, "fn bar() {}\n").unwrap();

        let input = format!("rename path={} line=1", f.to_str().unwrap());
        let (obj, _) =
            build_target(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap();

        assert_eq!(obj.get("column").and_then(|v| v.as_u64()), Some(0));
    }

    #[test]
    fn require_line_true_errors_without_line() {
        let dir = std::env::temp_dir().join("lmd_addr_reqline");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("c.rs");
        std::fs::write(&f, "fn baz() {}\n").unwrap();

        let input = format!("rename path={}", f.to_str().unwrap());
        let err =
            build_target(&DirectiveArgs::parse(&input), dir.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("line")), "got: {err:?}");
    }

    #[test]
    fn require_line_false_allows_path_only_whole_file() {
        // @reformat: reformat-by-path with no line= is valid; only the path is
        // resolved and no line/column keys are inserted.
        let dir = std::env::temp_dir().join("lmd_addr_pathonly");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("d.rs");
        std::fs::write(&f, "fn qux() {}\n").unwrap();

        let input = format!("path={}", f.to_str().unwrap());
        let (obj, abs) =
            build_target_with(&DirectiveArgs::parse(&input), dir.to_str().unwrap(), false)
                .unwrap();
        assert!(obj.get("line").is_none(), "no line key for path-only");
        assert!(obj.get("column").is_none(), "no column key for path-only");
        assert!(!abs.is_empty(), "abs_path must be set for path= addressing");
    }

    #[test]
    fn missing_path_and_name_errors() {
        let err = build_target(&DirectiveArgs::parse("rename"), ".").unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("path")), "got: {err:?}");
    }
}
