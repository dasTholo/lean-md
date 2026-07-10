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

/// Built-in `parallel-dispatch` fragment — single-source fan-out guidance
/// (when-to-use gate + fan-out rule + prompt structure + mandatory memory/
/// coordination block). Shared by the standalone `lmd-dispatching-parallel-agents`
/// skill and the SDD `parallel-dispatch` phase via `@include parallel-dispatch`.
const PARALLEL_DISPATCH: &str = include_str!("../content/core/_fragments/parallel-dispatch.lmd.md");

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

/// Fragment names that live inside a skill's `_includes/`, mapped to their owning
/// skill. They travel with that skill in the `kind=skills` pack (#727) — flat
/// global name, skill-scoped storage.
const SKILL_INCLUDES: &[(&str, &str)] = &[
    ("test-first-core", "lmd-test-driven-development"),
    ("skill-authoring-core", "lmd-writing-skills"),
    ("brainstorm-gate", "lmd-brainstorm"),
];

impl FragmentRegistry {
    pub fn with_builtins() -> Self {
        let mut builtins = HashMap::new();
        builtins.insert("hard-rules", HARD_RULES);
        builtins.insert("dispatch-contract", DISPATCH_CONTRACT);
        builtins.insert("parallel-dispatch", PARALLEL_DISPATCH);
        Self { builtins }
    }

    /// Three stages: cross-skill builtin → skill-local `_includes/` in the pack →
    /// jailed `<name>.lmd.md` file. The file stage stays user-extensible.
    pub fn resolve(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
        if let Some(content) = self.builtins.get(name) {
            return Ok((*content).to_string());
        }
        if let Some((_, skill)) = SKILL_INCLUDES.iter().find(|(n, _)| *n == name) {
            let rel = format!("{skill}/_includes/{name}.lmd.md");
            return crate::skill_source::read_skill_file(&rel, jail_root)
                .map_err(|e| ResolveError::Io(e.to_string()));
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
        // Post-slim invariant: hard-rules no longer names language-specific
        // backings (ctx_refactor/@symbol/@edit/ctx_search:symbol) inline —
        // those moved to `lang/<lang>` (Spec §7). hard-rules must instead
        // carry the pointers that delegate to them.
        assert!(
            out.contains("lang/"),
            "hard-rules must point to lang/<lang> for symbol/edit/reformat backings"
        );
        assert!(
            out.contains("tooling/mcp-tools"),
            "hard-rules must point to tooling/mcp-tools for I/O backings"
        );
    }

    #[test]
    fn test_first_core_resolves_through_pack_stage_not_builtins() {
        // #727 cut: test-first-core must come from the pack store at resolve
        // time, not from a compiled-in constant. Proof: point LEAN_MD_SKILLS_DIR
        // at a fabricated pack whose content differs from the real seed — if
        // resolve() were still reading a builtin HashMap entry, the env var
        // would be ignored and this content would never surface.
        let jail_root =
            std::env::temp_dir().join(format!("lmd_frag_pack_jail_{}", std::process::id()));
        let pack_root =
            std::env::temp_dir().join(format!("lmd_frag_pack_store_{}", std::process::id()));
        std::fs::create_dir_all(&jail_root).unwrap();
        let includes_dir = pack_root.join("lmd-test-driven-development/_includes");
        std::fs::create_dir_all(&includes_dir).unwrap();
        std::fs::write(
            includes_dir.join("test-first-core.lmd.md"),
            "fabricated pack-store test-first-core content\n",
        )
        .unwrap();

        crate::test_env::set_var(
            crate::skill_source::SKILLS_DIR_ENV,
            pack_root.to_str().unwrap(),
        );
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("test-first-core", &jail_root).unwrap();
        crate::test_env::remove_var(crate::skill_source::SKILLS_DIR_ENV);

        assert_eq!(
            out, "fabricated pack-store test-first-core content\n",
            "test-first-core must resolve from the pack store, not a builtin constant"
        );
    }

    #[test]
    fn hard_rules_dispatch_contract_and_parallel_dispatch_stay_builtin_resolved() {
        // These three are cross-skill lmd primitives — must resolve straight
        // from the built-in HashMap, with zero file I/O. A nonexistent
        // jail_root proves it: the file/pack stages would error on it, but
        // the builtin lookup short-circuits before ever touching jail_root.
        let jail_root = Path::new("/nonexistent/lmd_frag_builtin_probe/jail/root");
        let reg = FragmentRegistry::with_builtins();
        for name in ["hard-rules", "dispatch-contract", "parallel-dispatch"] {
            let out = reg.resolve(name, jail_root);
            assert!(
                out.is_ok(),
                "{name} must resolve without touching jail_root (builtin, no I/O)"
            );
        }
    }

    #[test]
    fn unknown_fragment_falls_back_to_jailed_file() {
        // Stage 3 (jailed `<name>.lmd.md` file) must survive the pack cut
        // untouched — it is the user-extensible layer, orthogonal to the #727
        // skill-local `_includes/` move.
        let dir =
            std::env::temp_dir().join(format!("lmd_frag_file_fallback_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("custom-fragment.lmd.md"), "from a jailed file\n").unwrap();
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("custom-fragment", &dir).unwrap();
        assert_eq!(out, "from a jailed file\n");
    }

    #[test]
    fn brainstorm_gate_resolves_through_pack_stage() {
        // #727 cut: brainstorm-gate is skill-owned content — it must resolve
        // through the SKILL_INCLUDES pack stage (lmd-brainstorm/_includes/…),
        // not a builtin. Same proof shape as test-first-core: a fabricated
        // pack-store content only surfaces if resolve() does live pack I/O.
        let jail_root =
            std::env::temp_dir().join(format!("lmd_frag_bg_jail_{}", std::process::id()));
        let pack_root =
            std::env::temp_dir().join(format!("lmd_frag_bg_pack_{}", std::process::id()));
        std::fs::create_dir_all(&jail_root).unwrap();
        let includes_dir = pack_root.join("lmd-brainstorm/_includes");
        std::fs::create_dir_all(&includes_dir).unwrap();
        std::fs::write(
            includes_dir.join("brainstorm-gate.lmd.md"),
            "fabricated pack-store brainstorm-gate content\n",
        )
        .unwrap();

        crate::test_env::set_var(
            crate::skill_source::SKILLS_DIR_ENV,
            pack_root.to_str().unwrap(),
        );
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("brainstorm-gate", &jail_root).unwrap();
        crate::test_env::remove_var(crate::skill_source::SKILLS_DIR_ENV);

        assert_eq!(
            out, "fabricated pack-store brainstorm-gate content\n",
            "brainstorm-gate must resolve from the pack store, not a builtin constant"
        );

        // Sanity check on the real seed content (debug fallback, no pack env
        // set): the HARD-GATE marker must still be present at its canonical path.
        let seed = reg.resolve("brainstorm-gate", Path::new(".")).unwrap();
        assert!(
            seed.contains("regardless of perceived simplicity"),
            "gate must carry the HARD-GATE marker"
        );
    }

    #[test]
    fn parallel_dispatch_matches_seed_file_on_disk() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let reg = FragmentRegistry::with_builtins();
        let disk = std::fs::read_to_string(
            std::path::Path::new(manifest).join("content/core/_fragments/parallel-dispatch.lmd.md"),
        )
        .unwrap();
        let builtin = reg.resolve("parallel-dispatch", Path::new(".")).unwrap();
        assert_eq!(builtin, disk, "parallel-dispatch drifted from seed file");
        assert!(
            builtin.contains("one dispatch per independent problem domain"),
            "fragment must carry the core-principle marker"
        );
        assert!(
            builtin.contains("ctx_agent action=handoff"),
            "fragment must carry the mandatory memory/coordination block"
        );
    }

    #[test]
    fn builtin_fragments_match_seed_files_on_disk() {
        // §8 #9 / #727 cut: only the two remaining cross-skill builtins
        // (hard-rules, dispatch-contract — parallel-dispatch has its own
        // drift test below) MUST be byte-identical to the canonical
        // lean-md/core seed files. test-first-core/skill-authoring-core/
        // brainstorm-gate moved to the SKILL_INCLUDES pack stage — they no
        // longer have a compiled-in constant to drift from.
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
    }

    #[test]
    fn hard_rules_slim() {
        let reg = FragmentRegistry::with_builtins();
        let hr = reg.resolve("hard-rules", Path::new(".")).unwrap();

        // The one non-redundant rule survives.
        assert!(
            hr.to_lowercase().contains("never native"),
            "must keep the never-native rule"
        );
        assert!(hr.contains("ctx_shell raw"), "must keep the never-raw rule");
        // The lean-ctx marker stays (builtin_resolves_before_file depends on it).
        assert!(hr.contains("lean-ctx"), "must keep the lean-ctx marker");
        // Now points to the concrete seeds instead of restating them.
        assert!(
            hr.contains("tooling/mcp-tools"),
            "must point to tooling/mcp-tools"
        );
        assert!(hr.contains("lang/"), "must point to lang/<lang>");
        // Redundant prose removed (now lives in lang/rust + tooling/mcp-tools).
        assert!(
            !hr.contains("prefer symbol-aware"),
            "redundant *.rs prose must be gone"
        );
        assert!(
            !hr.contains("reformat before"),
            "redundant reformat prose must be gone"
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
