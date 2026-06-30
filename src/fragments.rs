//! Fragment resolution: built-in-first, then a jailed `*.lmd.md` file fallback
//! (spec §3.3). Built-ins carry the canonical, logic-stable fragments (e.g. the
//! tool-discipline hard-rules) with zero disk cost; files override/extend them.

use std::collections::HashMap;
use std::path::Path;

/// Built-in `hard-rules` fragment — the canonical tool-discipline block that
/// goes into every dispatch (spec §3.3/§3.5). Kept short on purpose.
const HARD_RULES: &str = include_str!("../content/core/hard-rules.lmd.md");

/// Built-in `dispatch-contract` fragment (Spec §3.1, D-5/D-11). Block (b) of a
/// `@dispatch` render: tool-discipline + register/handoff baton. Ported from
/// `lean-md/core/dispatch-contract.lmd.md` (via `include_str!`, byte-stable #498).
/// `{{ role }}` / `{{ controller_id }}` stay verbatim — the `DispatchBridge`
/// (Phase 7C) substitutes them.
const DISPATCH_CONTRACT: &str = include_str!("../content/core/dispatch-contract.lmd.md");

/// Built-in `test-first-core` fragment — the TDD discipline trip-wires
/// (Iron Law + letter==spirit + red flags). Skill-owned seed, flat global name;
/// `@include test-first-core` pulls it into every isolated TDD phase (Spec E5).
const TEST_FIRST_CORE: &str =
    include_str!("../content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md");

/// Built-in `skill-authoring-core` fragment — the writing-skills discipline
/// trip-wires (Iron Law + letter==spirit + TDD mapping + WARUM pointer).
/// Skill-owned seed, flat global name; `@include skill-authoring-core` pulls it
/// into every isolated writing-skills phase.
const SKILL_AUTHORING_CORE: &str =
    include_str!("../content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md");

/// Built-in `brainstorm-gate` fragment — the HARD-GATE trip-wire that enforces
/// brainstorming discipline. Skill-owned seed, flat global name; `@include
/// brainstorm-gate` pulls it into every discipline phase of `lmd-brainstorm`.
const BRAINSTORM_GATE: &str =
    include_str!("../content/skills/lmd-brainstorm/_includes/brainstorm-gate.lmd.md");

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
        builtins.insert("test-first-core", TEST_FIRST_CORE);
        builtins.insert("skill-authoring-core", SKILL_AUTHORING_CORE);
        builtins.insert("brainstorm-gate", BRAINSTORM_GATE);
        Self { builtins }
    }

    pub fn resolve(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
        if let Some(content) = self.builtins.get(name) {
            return Ok((*content).to_string());
        }
        let candidate = jail_root.join(format!("{name}.lmd.md"));
        let resolved = crate::pathx::jail_path(&candidate, jail_root)
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
        // Parametrization stays verbatim — substitution is the DispatchBridge's job.
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
        assert!(
            out.contains("{{ crp }}"),
            "contract must carry the {{{{ crp }}}} placeholder"
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
        // D-8: serena/jetbrains were removed; the canon must no longer name them.
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
        // Today's backings must be named.
        assert!(
            out.contains("ctx_refactor"),
            "hard-rules must name ctx_refactor"
        );
        assert!(out.contains("@symbol"), "hard-rules must name @symbol");
        assert!(out.contains("@edit"), "hard-rules must name @edit");
        assert!(
            out.contains("ctx_search:symbol"),
            "hard-rules must name ctx_search:symbol for *.rs (@symbol backing)"
        );
    }

    #[test]
    fn brainstorm_gate_matches_seed_file_on_disk() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let reg = FragmentRegistry::with_builtins();
        let disk = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-brainstorm/_includes/brainstorm-gate.lmd.md"),
        )
        .unwrap();
        let builtin = reg.resolve("brainstorm-gate", Path::new(".")).unwrap();
        assert_eq!(builtin, disk, "brainstorm-gate drifted from seed file");
        assert!(
            builtin.contains("regardless of perceived simplicity"),
            "gate must carry the HARD-GATE marker"
        );
    }

    #[test]
    fn builtin_fragments_match_seed_files_on_disk() {
        // §8 #9: the built-in fragments MUST be byte-identical to the canonical
        // lean-md/core seed files. Reading the seed at test time (via the crate
        // manifest dir) catches any drift between the embedded const and the file.
        let manifest = env!("CARGO_MANIFEST_DIR"); // crate root
        let core = std::path::Path::new(manifest).join("content/core");
        let reg = FragmentRegistry::with_builtins();

        let hard_disk = std::fs::read_to_string(core.join("hard-rules.lmd.md")).unwrap();
        let hard_builtin = reg.resolve("hard-rules", Path::new(".")).unwrap();
        assert_eq!(hard_builtin, hard_disk, "hard-rules drifted from seed file");

        let disp_disk = std::fs::read_to_string(core.join("dispatch-contract.lmd.md")).unwrap();
        let disp_builtin = reg.resolve("dispatch-contract", Path::new(".")).unwrap();
        assert_eq!(
            disp_builtin, disp_disk,
            "dispatch-contract drifted from seed file"
        );

        let tfc_disk =
            std::fs::read_to_string(std::path::Path::new(manifest).join(
                "content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md",
            ))
            .unwrap();
        let tfc_builtin = reg.resolve("test-first-core", Path::new(".")).unwrap();
        assert_eq!(
            tfc_builtin, tfc_disk,
            "test-first-core drifted from seed file"
        );

        let sac_disk = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md"),
        )
        .unwrap();
        let sac_builtin = reg.resolve("skill-authoring-core", Path::new(".")).unwrap();
        assert_eq!(
            sac_builtin, sac_disk,
            "skill-authoring-core drifted from seed file"
        );
    }

    #[test]
    fn test_first_core_is_a_builtin_with_iron_law() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("test-first-core", Path::new(".")).unwrap();
        assert!(
            out.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"),
            "test-first-core must carry the Iron Law marker"
        );
        assert!(
            out.contains("Violating the letter of the rules is violating the spirit"),
            "test-first-core must carry the letter==spirit line"
        );
    }

    #[test]
    fn skill_authoring_core_is_a_builtin_with_iron_law() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("skill-authoring-core", Path::new(".")).unwrap();
        assert!(
            out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "skill-authoring-core must carry the Iron Law marker"
        );
        assert!(
            out.contains("Writing skills IS test-driven development"),
            "skill-authoring-core must state writing-skills-is-TDD"
        );
        assert!(
            out.contains("lmd-test-driven-development"),
            "skill-authoring-core must point to lmd-test-driven-development for the WARUM"
        );
    }

    #[test]
    fn test_first_core_carries_all_thirteen_red_flags() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("test-first-core", Path::new(".")).unwrap();
        // Original "Red Flags — STOP and Start Over": all 13 trip-wires (E12).
        for needle in [
            "Code before test",
            "Test after implementation",
            "passed immediately",
            "Can't explain why",
            "add the tests later",
            "Just this once",
            "already manually tested",
            "same purpose",
            "spirit not ritual",
            "Keep it as reference",
            "deleting is wasteful",
            "dogmatic",
            "different because",
        ] {
            assert!(
                out.contains(needle),
                "test-first-core missing red flag '{needle}': {out}"
            );
        }
        // Core principle restored alongside the Iron Law.
        assert!(
            out.contains("didn't watch the test fail"),
            "test-first-core must carry the core principle: {out}"
        );
    }
}
