//! Fragment resolution: built-in-first, then a jailed `*.lmd.md` file fallback
//! (spec §3.3). Built-ins carry the canonical, logic-stable fragments (e.g. the
//! tool-discipline hard-rules) with zero disk cost; files override/extend them.

use std::collections::HashMap;
use std::path::Path;

/// Built-in `hard-rules` fragment — the canonical tool-discipline block that
/// goes into every dispatch (spec §3.3/§3.5). Kept short on purpose.
const HARD_RULES: &str = "\
# Hard Rules (lmd built-in)
- I/O only via lean-ctx MCP tools (ctx_read/ctx_search/ctx_tree/ctx_shell).
- Never use native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
- For *.rs prefer symbol-aware tools: navigate & refactor via
  ctx_refactor / ctx_symbol (@symbol) — rename/move/extract over hand edits.
- Plain @edit / ctx_edit only for non-symbol changes; reformat before
  commit via ctx_refactor action=reformat.
";

/// Built-in `dispatch-contract` fragment (Spec §3.1, D-5/D-11). Block (b) of a
/// `@dispatch` render: tool-discipline + register/handoff baton. Portiert aus
/// `.claude/rules/subagent-multi-agent.md`. `{{ role }}` / `{{ controller_id }}`
/// bleiben verbatim — die `DispatchBridge` (Phase 7C) substituiert sie.
const DISPATCH_CONTRACT: &str = "\
## lean-ctx Subagent Contract (MANDATORY)
You run in an isolated context. Before any other action:
1. ctx_agent action=register agent_type=subagent role={{ role }}
   (the controller's cache is already shared — no ctx_share pull, just ctx_read)

@include hard-rules

Tool discipline:
- Under tool_profile=power ALL lean-ctx tools are DIRECT — call them DIRECTLY. If one
  shows up deferred, run ToolSearch(query=\"select:<tool>\") FIRST, then call it. NEVER
  wrap a tool in ctx_call.
- NEVER fresh, NEVER raw — to re-read your own edits use ctx_delta or ctx_read mode=diff.
- Search → ctx_search (never grep/rg); read files → ctx_read (never cat).
- Rust (*.rs) edits → native Edit / ctx_edit; symbol nav/refactor → ctx_refactor / @symbol.
- git commit: PLAIN git commit -m \"subject\" -m \"trailer\" — NEVER a heredoc / $( ) subshell.

On finish:
- ctx_agent action=post category=<status|finding> message=\"<summary>\"
- ctx_agent action=handoff to_agent={{ controller_id }} message=\"<baton>\"
- ctx_knowledge action=remember for any durable fact/gotcha
Report final status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
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
        builtins.insert("dispatch-contract", DISPATCH_CONTRACT);
        Self { builtins }
    }

    pub fn resolve(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
        if let Some(content) = self.builtins.get(name) {
            return Ok((*content).to_string());
        }
        let candidate = jail_root.join(format!("{name}.lmd.md"));
        let resolved = crate::core::pathjail::jail_path(&candidate, jail_root)
            .map_err(|_| ResolveError::Jail(format!("{name} escapes jail")))?;
        if !resolved.exists() {
            return Err(ResolveError::NotFound(name.to_string()));
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

    #[test]
    fn dispatch_contract_is_a_builtin_with_placeholders() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("dispatch-contract", Path::new(".")).unwrap();
        // Parametrisierung bleibt verbatim — Substitution ist Sache der DispatchBridge.
        assert!(
            out.contains("{{ role }}"),
            "contract must carry the {{{{ role }}}} placeholder"
        );
        assert!(
            out.contains("{{ controller_id }}"),
            "contract must carry the {{{{ controller_id }}}} placeholder"
        );
        // Kanonische Bausteine (Spec §3.1): register-Zeile + Baton + Disziplin-Verweis.
        assert!(
            out.contains("ctx_agent"),
            "contract must instruct ctx_agent register/handoff"
        );
        assert!(
            out.contains("hard-rules"),
            "contract must compose hard-rules (@include)"
        );
        assert!(
            out.contains("NEVER"),
            "contract must carry the tool-discipline guardrails"
        );
    }

    #[test]
    fn absolute_path_name_is_jailed() {
        // M-1 hardening: an absolute name must not escape the jail.
        // jail_path catches /etc/passwd → not inside jail_root → Err → Jail variant.
        let reg = FragmentRegistry::with_builtins();
        let err = reg.resolve("/etc/passwd", Path::new(".")).unwrap_err();
        assert!(
            matches!(err, ResolveError::Jail(_)),
            "absolute name must produce Jail error, got: {err:?}"
        );
    }

    #[test]
    fn hard_rules_has_no_stale_backings() {
        // D-8: serena/jetbrains wurden entfernt; der Kanon darf sie nicht mehr nennen.
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("hard-rules", Path::new(".")).unwrap();
        assert!(
            !out.contains("serena"),
            "stale backing 'serena' in hard-rules"
        );
        assert!(
            !out.to_lowercase().contains("jetbrains"),
            "stale backing 'jetbrains' in hard-rules"
        );
        // Die heutigen Backings müssen genannt sein.
        assert!(
            out.contains("ctx_refactor"),
            "hard-rules must name ctx_refactor"
        );
        assert!(out.contains("@symbol"), "hard-rules must name @symbol");
        assert!(out.contains("@edit"), "hard-rules must name @edit");
        assert!(
            out.contains("ctx_symbol"),
            "hard-rules must name ctx_symbol for *.rs"
        );
    }
}
