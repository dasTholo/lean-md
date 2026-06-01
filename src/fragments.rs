//! Fragment resolution: built-in-first, then a jailed `*.lmd.md` file fallback
//! (spec §3.3). Built-ins carry the canonical, logic-stable fragments (e.g. the
//! tool-discipline hard-rules) with zero disk cost; files override/extend them.

use std::collections::HashMap;
use std::path::Path;

/// Built-in `hard-rules` fragment — the canonical tool-discipline block that
/// goes into every dispatch (spec §3.3/§3.5). Kept short on purpose.
const HARD_RULES: &str = "\
# Hard Rules (lmd built-in)
- I/O only via lean-ctx MCP tools (ctx_read/ctx_search/ctx_tree/ctx_shell), serena (symbols), jetbrains (reformat).
- Never use native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
- Edit *.rs only via serena symbol tools.
";

#[derive(Debug)]
pub enum ResolveError {
    NotFound(String),
    Jail(String),
    Io(String),
}

/// Built-in-first fragment registry. `resolve` checks the embedded built-ins,
/// then falls back to a jailed `<name>.lmd.md` file under `jail_root`.
pub struct FragmentRegistry {
    builtins: HashMap<&'static str, &'static str>,
}

impl FragmentRegistry {
    pub fn with_builtins() -> Self {
        let mut builtins = HashMap::new();
        builtins.insert("hard-rules", HARD_RULES);
        Self { builtins }
    }

    pub fn resolve(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
        if let Some(content) = self.builtins.get(name) {
            return Ok((*content).to_string());
        }
        // Reject names with path-traversal components before touching the filesystem.
        if Path::new(name).components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        }) {
            return Err(ResolveError::Jail(format!("{name} escapes jail")));
        }
        let candidate = jail_root.join(format!("{name}.lmd.md"));
        let jail = jail_root
            .canonicalize()
            .map_err(|e| ResolveError::Jail(format!("jail root: {e}")))?;
        let resolved = candidate
            .canonicalize()
            .map_err(|_| ResolveError::NotFound(name.to_string()))?;
        if !resolved.starts_with(&jail) {
            return Err(ResolveError::Jail(format!("{name} escapes jail")));
        }
        std::fs::read_to_string(&resolved).map_err(|e| ResolveError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn builtin_resolves_before_file() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("hard-rules", Path::new(".")).unwrap();
        assert!(
            out.contains("lean-ctx"),
            "built-in hard-rules must mention lean-ctx"
        );
    }

    #[test]
    fn unknown_fragment_errors() {
        let reg = FragmentRegistry::with_builtins();
        let err = reg.resolve("does-not-exist", Path::new(".")).unwrap_err();
        assert!(matches!(err, ResolveError::NotFound(_)));
    }

    #[test]
    fn file_fallback_within_jail() {
        let dir = std::env::temp_dir().join("lmd_frag_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("greeting.lmd.md"), "hello from file\n").unwrap();
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("greeting", &dir).unwrap();
        assert_eq!(out, "hello from file\n");
    }

    #[test]
    fn jail_blocks_escape() {
        let reg = FragmentRegistry::with_builtins();
        let err = reg.resolve("../etc/passwd", Path::new(".")).unwrap_err();
        assert!(matches!(err, ResolveError::Jail(_)));
    }
}
