//! Pre-flight path helpers — generic stdlib, no lean-ctx secret knowledge.
//! These are NOT the authoritative jail: every outbound ctx_* call is jailed
//! server-side (spec §6, bucket 3). We canonicalize + prefix-check before the
//! wire call so obvious escapes fail fast and locally.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Canonicalize `candidate` and ensure it stays within `jail_root`.
/// Mirrors the old `core::pathjail::jail_path` contract: walk up to the nearest
/// existing ancestor, canonicalize it (resolves `..` and symlinks), then
/// re-append the non-existent remainder. This catches escapes via `..` even
/// when the target file does not exist yet (a parent-only shortcut does not).
pub fn jail_path(candidate: &Path, jail_root: &Path) -> Result<PathBuf, String> {
    let root = std::fs::canonicalize(jail_root)
        .map_err(|e| format!("jail root not canonicalizable: {e}"))?;
    let (base, remainder) = canonicalize_existing_ancestor(candidate, &root)?;
    let mut abs = base;
    for part in remainder.iter().rev() {
        abs.push(part);
    }
    if abs.starts_with(&root) {
        Ok(abs)
    } else {
        Err(format!("path escapes jail: {}", candidate.display()))
    }
}

/// Walk `path` upward until an existing ancestor is found, canonicalize it, and
/// return it together with the popped (non-existent) tail components in pop
/// order. Relative paths are resolved against `fallback_root` first.
fn canonicalize_existing_ancestor(
    path: &Path,
    fallback_root: &Path,
) -> Result<(PathBuf, Vec<OsString>), String> {
    let mut cur = if path.is_absolute() {
        path.to_path_buf()
    } else {
        fallback_root.join(path)
    };
    let mut remainder: Vec<OsString> = Vec::new();
    loop {
        if cur.exists() {
            let canon = std::fs::canonicalize(&cur).map_err(|e| format!("{e}"))?;
            return Ok((canon, remainder));
        }
        match cur.file_name() {
            Some(name) => remainder.push(name.to_os_string()),
            None => return Err(format!("path has no existing ancestor: {}", path.display())),
        }
        if !cur.pop() {
            return Err(format!("path has no existing ancestor: {}", path.display()));
        }
    }
}

/// Resolve a user-facing `raw` path against `project_root` (or `shell_cwd`).
/// `"."`/`""` pass through unchanged (the wire call pins them to project_root).
/// Mirrors the old `core::path_resolve::resolve_tool_path` contract.
pub fn resolve_tool_path(
    project_root: Option<&str>,
    shell_cwd: Option<&str>,
    raw: &str,
) -> Result<String, String> {
    if raw.is_empty() || raw == "." {
        return Ok(raw.to_string());
    }
    let p = Path::new(raw);
    if p.is_absolute() {
        return Ok(raw.to_string());
    }
    let base = project_root.or(shell_cwd).unwrap_or(".");
    Ok(Path::new(base).join(raw).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jail_blocks_escape() {
        let root = std::env::temp_dir().join("pathx_jail_root");
        std::fs::create_dir_all(&root).unwrap();
        let err = jail_path(&root.join("../etc/passwd"), &root).unwrap_err();
        assert!(err.contains("escapes"), "got: {err}");
    }

    #[test]
    fn jail_allows_inside() {
        let root = std::env::temp_dir().join("pathx_jail_ok");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "x").unwrap();
        let p = jail_path(&root.join("a.txt"), &root).unwrap();
        assert!(p.starts_with(std::fs::canonicalize(&root).unwrap()));
    }

    #[test]
    fn jail_allows_nonexistent_child() {
        let root = std::env::temp_dir().join("pathx_jail_newchild");
        std::fs::create_dir_all(&root).unwrap();
        let p = jail_path(&root.join("new").join("file.txt"), &root).unwrap();
        assert!(p.to_string_lossy().contains("file.txt"));
    }

    #[test]
    fn resolve_passes_dot_through() {
        assert_eq!(resolve_tool_path(Some("/tmp"), None, ".").unwrap(), ".");
    }

    #[test]
    fn resolve_joins_relative_against_root() {
        let out = resolve_tool_path(Some("/tmp/proj"), None, "src/x.rs").unwrap();
        assert!(out.ends_with("src/x.rs"));
    }
}
