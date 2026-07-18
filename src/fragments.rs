//! Fragment resolution: built-in-first, then a jailed `*.lmd.md` file fallback
//! (spec §3.3). Built-ins carry the canonical, logic-stable fragments (e.g. the
//! tool-discipline hard-rules) with zero disk cost. A project EXTENDS any fragment
//! by dropping `<name>.ext.lmd.md` into `contracts_dir` — `resolve` appends it after
//! the base. It never overrides the built-in; an absent or inert `.ext` leaves the
//! output byte-identical (#498).

use std::collections::HashMap;
use std::path::Path;

/// Where project seeds materialise and where `<name>.ext.lmd.md` is looked up —
/// deliberately NOT jail_root, which is the fragment file fallback's home.
const CONTRACTS_DIR: &str = ".lean-ctx/lean-md";

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

    /// Base resolution — three stages: cross-skill builtin → skill-local `_includes/`
    /// in the pack → jailed `<name>.lmd.md` file. The file stage stays user-extensible.
    fn base(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
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

    /// Project extension for `name`, composed onto the resolved base. `None` when absent
    /// or inert. Lives in `contracts_dir` (where the seeds materialise), NOT at jail_root
    /// like the fragment file fallback — the two paths are deliberately different.
    fn ext(&self, name: &str, jail_root: &Path) -> Option<String> {
        let candidate = jail_root.join(format!("{CONTRACTS_DIR}/{name}.ext.lmd.md"));
        let resolved = crate::pathx::jail_path(&candidate, jail_root).ok()?;
        let raw = std::fs::read_to_string(&resolved).ok()?;
        (!strip_html_comments(&raw).trim().is_empty()).then_some(raw)
    }

    /// Resolve `name` to its final text: the base fragment plus an optional project
    /// `<name>.ext.lmd.md` appended after it. An absent or inert `.ext` leaves the
    /// output byte-identical to the base (#498).
    pub fn resolve(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
        let base = self.base(name, jail_root)?;
        Ok(match self.ext(name, jail_root) {
            Some(ext) => format!("{base}\n{ext}"),
            None => base,
        })
    }
}

/// Remove `<!-- … -->` spans so an all-comment seed reads as inert.
fn strip_html_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out, // unterminated comment swallows the tail
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Jail root with an optional `<name>.ext.lmd.md` in contracts_dir — the same layout
    /// `materialize_contracts` produces and `ext_fixture` (bridges/dispatch.rs) uses.
    fn ext_root(tag: &str, name: &str, body: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lmd_ext_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let contracts = dir.join(".lean-ctx/lean-md");
        std::fs::create_dir_all(&contracts).unwrap();
        if let Some(b) = body {
            std::fs::write(contracts.join(format!("{name}.ext.lmd.md")), b).unwrap();
        }
        dir
    }

    #[test]
    fn ext_composes_onto_any_builtin_fragment() {
        // hard-rules.ext is a dead file today — nothing reads it. That is the bug.
        let dir = ext_root("generic", "hard-rules", Some("PROJECT RULE: no cd\n"));
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("hard-rules", &dir).unwrap();
        assert!(out.contains("lean-ctx"), "built-in body must survive");
        assert!(
            out.contains("PROJECT RULE: no cd"),
            "ext must be appended: {out}"
        );
        assert!(
            out.find("lean-ctx").unwrap() < out.find("PROJECT RULE").unwrap(),
            "ext comes AFTER the built-in"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        // `base` for the same reason as builtin_fragments_match_seed_files_on_disk.
        let builtin = reg.base("parallel-dispatch", Path::new(".")).unwrap();
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

        // `base`, not `resolve`: the claim is about the built-in constant vs. the seed
        // file. `resolve` would additionally compose this repo's own project `.ext`, so a
        // local extension would read as built-in drift.
        let hard_disk = std::fs::read_to_string(core.join("hard-rules.lmd.md")).unwrap();
        let hard_builtin = reg.base("hard-rules", Path::new(".")).unwrap();
        assert_eq!(hard_builtin, hard_disk, "hard-rules drifted from seed file");

        let disp_disk = std::fs::read_to_string(core.join("dispatch-contract.lmd.md")).unwrap();
        let disp_builtin = reg.base("dispatch-contract", Path::new(".")).unwrap();
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

    #[test]
    fn ext_at_the_jail_root_is_ignored() {
        // Pins the path decision: the fragment file fallback lives at jail_root, the .ext
        // lives in contracts_dir. Reading the ext at jail_root would make every shipped
        // seed a dead file — the exact defect this task removes.
        let dir = ext_root("wrongpath", "hard-rules", None);
        std::fs::write(dir.join("hard-rules.ext.lmd.md"), "WRONG PLACE\n").unwrap();
        let reg = FragmentRegistry::with_builtins();
        assert!(
            !reg.resolve("hard-rules", &dir)
                .unwrap()
                .contains("WRONG PLACE")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hard_rules_ext_inherits_into_dispatch_contract_via_include() {
        // The actual lever of the generalisation: a project rule lives in ONE place and
        // reaches EVERY dispatch through the contract's `@include hard-rules` — with no
        // entry in dispatch-contract.ext. Must be proven, not assumed.
        let dir = ext_root("inherit", "hard-rules", Some("PROJECT RULE: inherited\n"));
        assert!(
            !dir.join(".lean-ctx/lean-md/dispatch-contract.ext.lmd.md")
                .exists()
        );

        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nwork\n@phase-end\n@dispatch phase=t\n";
        let out =
            crate::skills::render_source_with_phase(src, None, None, None, dir.clone()).unwrap();
        assert!(
            out.contains("PROJECT RULE: inherited"),
            "hard-rules.ext must reach the dispatch contract via @include hard-rules: {out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_inert_ext_leaves_the_output_byte_identical() {
        // #498: an untouched seed must not change a single byte. This is the regression
        // guard against exactly what the stale dispatch-contract.ext does today.
        let seed = crate::seeds::PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "hard-rules.ext.lmd.md")
            .map(|(_, c)| *c)
            .expect("hard-rules.ext must be registered");
        let with_dir = ext_root("inert", "hard-rules", Some(seed));
        let without_dir = ext_root("inert_absent", "hard-rules", None);
        let reg = FragmentRegistry::with_builtins();
        assert_eq!(
            reg.resolve("hard-rules", &with_dir).unwrap(),
            reg.resolve("hard-rules", &without_dir).unwrap(),
            "the shipped hard-rules.ext seed must be inert"
        );
        let _ = std::fs::remove_dir_all(&with_dir);
        let _ = std::fs::remove_dir_all(&without_dir);
    }

    #[test]
    fn the_parallel_dispatch_seed_is_inert_too() {
        let seed = crate::seeds::PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "parallel-dispatch.ext.lmd.md")
            .map(|(_, c)| *c)
            .expect("parallel-dispatch.ext must be registered");
        let with_dir = ext_root("pd_inert", "parallel-dispatch", Some(seed));
        let without_dir = ext_root("pd_absent", "parallel-dispatch", None);
        let reg = FragmentRegistry::with_builtins();
        assert_eq!(
            reg.resolve("parallel-dispatch", &with_dir).unwrap(),
            reg.resolve("parallel-dispatch", &without_dir).unwrap()
        );
        let _ = std::fs::remove_dir_all(&with_dir);
        let _ = std::fs::remove_dir_all(&without_dir);
    }

    #[test]
    fn a_markdown_only_ext_is_not_inert() {
        // The live defect, pinned: `#` lines are headings, not comments — they survive
        // strip_html_comments and get appended. A seed that WANTS to be empty must say so
        // in HTML. This is why the shipped seeds are HTML comments.
        let dir = ext_root("md", "hard-rules", Some("# a heading\n"));
        let reg = FragmentRegistry::with_builtins();
        assert!(
            reg.resolve("hard-rules", &dir)
                .unwrap()
                .contains("# a heading")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ext_takes_part_in_placeholder_substitution() {
        let dir = ext_root("ph", "dispatch-contract", Some("role is {{ role }}\n"));
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nwork\n@phase-end\n@dispatch phase=t role=review\n";
        let out =
            crate::skills::render_source_with_phase(src, None, None, None, dir.clone()).unwrap();
        assert!(
            out.contains("role is review"),
            "ext must see substitution: {out}"
        );
        assert!(!out.contains("{{ role }}"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_ext_changes_nothing() {
        let dir = ext_root("absent", "hard-rules", None);
        let reg = FragmentRegistry::with_builtins();
        assert_eq!(
            reg.resolve("hard-rules", &dir).unwrap(),
            reg.resolve("hard-rules", Path::new(".")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ext_lookup_is_jailed() {
        // Both the base lookup and the new ext lookup run through pathx::jail_path.
        let reg = FragmentRegistry::with_builtins();
        let err = reg.resolve("../etc/passwd", Path::new(".")).unwrap_err();
        assert!(matches!(err, ResolveError::Jail(_)));
    }

    #[test]
    fn dispatch_bridge_reads_no_file_itself() {
        // The special path is gone: composition belongs to the registry, so it applies to
        // every fragment instead of exactly one name.
        let src = include_str!("bridges/dispatch.rs");
        assert!(
            !src.contains("fn contract_ext"),
            "contract_ext must move to the registry"
        );
        assert!(
            !src.contains("read_to_string"),
            "the bridge must not do file I/O"
        );
    }

    #[test]
    fn no_ext_seed_for_file_backed_fragments() {
        // lang/rust and tooling/mcp-tools have no built-in — their materialised file IS the
        // source and is edited directly. A second way to the same goal would be the bug.
        for (rel, _) in crate::seeds::PROJECT_SEEDS {
            assert!(
                !rel.starts_with("lang/") || !rel.contains(".ext."),
                "no .ext seed for a file-backed fragment: {rel}"
            );
            assert!(
                !rel.starts_with("tooling/") || !rel.contains(".ext."),
                "no .ext seed for a file-backed fragment: {rel}"
            );
        }
    }

    #[test]
    fn both_new_ext_seeds_are_registered() {
        let paths: Vec<&str> = crate::seeds::PROJECT_SEEDS
            .iter()
            .map(|(p, _)| *p)
            .collect();
        assert!(paths.contains(&"hard-rules.ext.lmd.md"));
        assert!(paths.contains(&"parallel-dispatch.ext.lmd.md"));
    }

    #[test]
    fn a_stale_markdown_ext_is_caught_by_the_refresh_before_it_is_composed() {
        let root = std::env::temp_dir().join(format!("lmd_ext_couple_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        crate::seeds::materialize_contracts(&root, dir, false).unwrap();
        let ext = root.join(dir).join("dispatch-contract.ext.lmd.md");
        std::fs::write(&ext, "# lean-md dispatch-contract extension\n").unwrap(); // the live defect

        // No lock → unknown provenance → preserved + .new, never silently composed away.
        let report = crate::seeds::refresh_contracts(&root, dir).unwrap();
        assert!(
            report
                .preserved
                .iter()
                .any(|p| p.ends_with("dispatch-contract.ext.lmd.md"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- migrated from bridges/dispatch.rs: the `.ext` path this task generalised ---

    /// Build a jail root under the temp dir; `ext` = optional `.ext.lmd.md` content.
    fn ext_fixture(tag: &str, ext: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lmd_dispatch_ext_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".lean-ctx/lean-md")).unwrap();
        if let Some(body) = ext {
            std::fs::write(
                dir.join(".lean-ctx/lean-md/dispatch-contract.ext.lmd.md"),
                body,
            )
            .unwrap();
        }
        dir
    }

    fn render_dispatch_in(jail_root: std::path::PathBuf) -> String {
        use crate::engine::{EngineContext, render_body};
        use crate::header::LeanMdHeader;
        use std::rc::Rc;
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), jail_root));
        render_body(
            &ctx,
            "@phase \"P\"\nDo the work.\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n",
        )
    }

    #[test]
    fn dispatch_ext_rule_appears_after_contract() {
        let dir = ext_fixture("rule", Some("- PROJECT_EXT_RULE_XYZ: always ship green.\n"));
        let out = render_dispatch_in(dir);
        let rule = out
            .find("PROJECT_EXT_RULE_XYZ")
            .unwrap_or_else(|| panic!("ext rule missing: {out}"));
        let contract = out
            .find("Subagent Contract")
            .unwrap_or_else(|| panic!("contract missing: {out}"));
        let task = out
            .find("## Task (phase-isolated)")
            .unwrap_or_else(|| panic!("task header missing: {out}"));
        assert!(contract < rule, "ext must follow the contract: {out}");
        assert!(rule < task, "ext must precede the task block: {out}");
    }

    #[test]
    fn dispatch_untouched_ext_seed_is_byte_stable() {
        // The shipped seed is comments-only → inert → byte-identical to no-ext (#498).
        let seed = include_str!("../content/templates/dispatch-contract.ext.lmd.md");
        let with_seed = render_dispatch_in(ext_fixture("seed", Some(seed)));
        let without = render_dispatch_in(ext_fixture("seed_absent", None));
        assert_eq!(with_seed, without, "untouched seed must not alter output");

        // Pinned decision: an unterminated `<!--` swallows the tail → file reads inert.
        let unterminated =
            render_dispatch_in(ext_fixture("unterminated", Some("<!-- oops\nRULE\n")));
        assert_eq!(
            unterminated, without,
            "unterminated comment must render the ext inert"
        );
    }

    #[test]
    fn dispatch_absent_ext_is_unchanged() {
        let no_file = render_dispatch_in(ext_fixture("absent_file", None));
        let no_dir = {
            let dir = std::env::temp_dir().join("lmd_dispatch_ext_absent_dir");
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            render_dispatch_in(dir)
        };
        assert_eq!(no_file, no_dir, "absent ext must leave output unchanged");
        assert!(
            no_file.contains("role=dev"),
            "dispatch still renders: {no_file}"
        );
    }

    #[test]
    fn dispatch_ext_jail_escape_still_rejected() {
        let outside = std::env::temp_dir().join("lmd_dispatch_ext_outside.lmd.md");
        std::fs::write(&outside, "- EXT_JAIL_ESCAPE_SECRET\n").unwrap();
        let dir = ext_fixture("escape", None);
        let link = dir.join(".lean-ctx/lean-md/dispatch-contract.ext.lmd.md");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let out = render_dispatch_in(dir);
        assert!(
            !out.contains("EXT_JAIL_ESCAPE_SECRET"),
            "symlinked out-of-jail ext must not be read: {out}"
        );
    }
}
