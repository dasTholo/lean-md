//! Project materialization of lang/tooling/.ext seeds (Spec §5.4, layer B).
//! Seeds are binary-embedded (`include_str!`) and copied into the project's
//! `contracts_dir` in one of three modes:
//!
//! * absent-only (`materialize_contracts(.., force=false)`) — the install default
//!   for a fresh target; never touches an existing file, so seeds age silently.
//! * `force` (`materialize_contracts(.., force=true)`) — the deliberate hammer:
//!   overwrites unconditionally, local changes included.
//! * lock-based (`refresh_contracts`) — uses `.lean-ctx/lean-md.lock` to tell a
//!   stale-but-untouched seed (heal it) from a user-edited one (`.new` beside it).
//!
//! Materializing a seed does not by itself decide what a render resolves to —
//! for the resolution order (built-in vs. project file) see `fragments.rs`. The
//! `*.ext.lmd.md` seeds EXTEND their built-in fragment: `FragmentRegistry::resolve`
//! appends the `.ext` after the built-in body, it never replaces it. They ship inert
//! (HTML comments only) so an untouched seed keeps every render byte-stable (#498).

use std::path::{Path, PathBuf};

/// Project-local macro library (`test`/`commit`/`tdd`) imported by every
/// generated `.lmd.md` plan. Module-level (not test-only) so Subplan-4-Task-2
/// can register it as a `PROJECT_SEEDS` entry without moving it.
const PLAN_RECIPES: &str = include_str!("../content/templates/plan-recipes.lmd.md");

/// Self-documenting `.lmd.md` plan skeleton (meta-head + one real `@phase`
/// example). Module-level for the same reason as `PLAN_RECIPES`.
const PLAN_TEMPLATE: &str = include_str!("../content/templates/plan-template.lmd.md");

/// (relative target path under contracts_dir, embedded content).
pub const PROJECT_SEEDS: &[(&str, &str)] = &[
    (
        "lang/rust.lmd.md",
        include_str!("../content/lang/rust.lmd.md"),
    ),
    (
        "tooling/mcp-tools.lmd.md",
        include_str!("../content/tooling/mcp-tools.lmd.md"),
    ),
    (
        "dispatch-contract.ext.lmd.md",
        include_str!("../content/templates/dispatch-contract.ext.lmd.md"),
    ),
    (
        "hard-rules.ext.lmd.md",
        include_str!("../content/templates/hard-rules.ext.lmd.md"),
    ),
    (
        "parallel-dispatch.ext.lmd.md",
        include_str!("../content/templates/parallel-dispatch.ext.lmd.md"),
    ),
    ("plan-recipes.lmd.md", PLAN_RECIPES),
    ("plan-template.lmd.md", PLAN_TEMPLATE),
];

/// Materialize embedded project seeds into `<project_root>/<contracts_dir>`.
/// `force=false` is absent-only (idempotent, never clobbers user edits); `force=true`
/// overwrites an existing target to refresh a stale derived seed after the embedded
/// copy changed. Returns the paths actually written.
pub fn materialize_contracts(
    project_root: &Path,
    contracts_dir: &str,
    force: bool,
) -> std::io::Result<Vec<PathBuf>> {
    let base = project_root.join(contracts_dir);
    let mut written = Vec::new();
    for (rel, content) in PROJECT_SEEDS {
        let target = base.join(rel);
        if !force && target.exists() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, content)?;
        written.push(target);
    }
    Ok(written)
}

/// What a `refresh_contracts` run did that the user may want to know about.
#[derive(Default)]
pub struct RefreshReport {
    /// Stale but untouched seeds that were silently updated to the embedded copy.
    pub healed: Vec<PathBuf>,
    /// Seeds carrying local changes (or unknown provenance): left alone, embedded
    /// copy written beside them as `<target>.new`.
    pub preserved: Vec<PathBuf>,
}

impl RefreshReport {
    /// Nothing happened that is worth a word to the user.
    pub fn is_quiet(&self) -> bool {
        self.healed.is_empty() && self.preserved.is_empty()
    }
}

/// Lock key for a seed: paths in the lock are relative to `.lean-ctx/`, the
/// directory `sha256sum -c` runs in.
fn lock_key(contracts_dir: &str, rel: &str) -> String {
    let dir = contracts_dir
        .strip_prefix(".lean-ctx/")
        .unwrap_or(contracts_dir)
        .trim_end_matches('/');
    if dir.is_empty() {
        rel.to_string()
    } else {
        format!("{dir}/{rel}")
    }
}

/// Lock-based refresh — the third mode beside absent-only and `force`.
///
/// Absent-only lets seeds age; `force` clobbers real local work. The lock records
/// the seed hash this project was materialized with, which is what separates
/// "stale but untouched" (heal it) from "the user changed it" (never touch it,
/// drop the new copy beside it as `.new` and say so). A seed with no lock entry
/// has unknown provenance and is treated as user-owned.
pub fn refresh_contracts(
    project_root: &Path,
    contracts_dir: &str,
) -> std::io::Result<RefreshReport> {
    let base = project_root.join(contracts_dir);
    let mut lock = crate::lock::Lock::load(project_root);
    let mut report = RefreshReport::default();
    let mut lock_dirty = false;

    for (rel, content) in PROJECT_SEEDS {
        let target = base.join(rel);
        let key = lock_key(contracts_dir, rel);
        let embedded_hex = crate::hashx::sha256_hex(content.as_bytes());

        let Ok(local) = std::fs::read(&target) else {
            // Absent target: an install, not a user edit — write it and record it.
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, content)?;
            lock.set(&key, &embedded_hex);
            lock_dirty = true;
            continue;
        };
        let local_hex = crate::hashx::sha256_hex(&local);

        if local_hex == embedded_hex {
            // Already current. Record provenance if the lock did not know it yet.
            if lock.get(&key) != Some(embedded_hex.as_str()) {
                lock.set(&key, &embedded_hex);
                lock_dirty = true;
            }
            continue;
        }

        if lock.get(&key) == Some(local_hex.as_str()) {
            // Local matches what we last wrote → untouched, only stale → heal it.
            std::fs::write(&target, content)?;
            lock.set(&key, &embedded_hex);
            lock_dirty = true;
            report.healed.push(target);
        } else {
            // Local edit, or no lock entry (unknown provenance): never overwrite.
            let mut new_name = target.file_name().unwrap_or_default().to_os_string();
            new_name.push(".new");
            std::fs::write(target.with_file_name(new_name), content)?;
            report.preserved.push(target);
        }
    }

    if lock_dirty {
        lock.save(project_root)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Macro names defined in a plan-recipes source (`@define NAME(...)`).
    fn defined_macro_names(src: &str) -> std::collections::HashSet<String> {
        src.lines()
            .filter_map(|l| l.trim_start().strip_prefix("@define "))
            .filter_map(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
            .collect()
    }

    #[test]
    fn seeds_are_non_empty_and_unique() {
        assert!(!PROJECT_SEEDS.is_empty());
        let mut paths: Vec<&str> = PROJECT_SEEDS.iter().map(|(p, _)| *p).collect();
        let n = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), n, "duplicate seed target paths");
        for (_, content) in PROJECT_SEEDS {
            assert!(
                !content.trim().is_empty(),
                "embedded seed must be non-empty"
            );
        }
    }

    #[test]
    fn project_seeds_materialize() {
        let root = std::env::temp_dir().join(format!("lmd_pseeds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let written = materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        let base = root.join(".lean-ctx/lean-md");
        assert!(
            base.join("plan-recipes.lmd.md").exists(),
            "plan-recipes must materialize at root"
        );
        assert!(
            base.join("plan-template.lmd.md").exists(),
            "plan-template must materialize at root"
        );
        assert!(written.iter().any(|p| p.ends_with("plan-recipes.lmd.md")));

        // Absent-only: a second run writes nothing new.
        let again = materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        assert!(
            again.is_empty(),
            "second run must be idempotent (absent-only)"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn materialize_writes_then_is_idempotent() {
        let root = std::env::temp_dir().join(format!("lmd_seeds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";

        let first = materialize_contracts(&root, dir, false).unwrap();
        assert_eq!(
            first.len(),
            PROJECT_SEEDS.len(),
            "first run writes all seeds"
        );
        for (rel, _) in PROJECT_SEEDS {
            assert!(root.join(dir).join(rel).exists(), "seed not written: {rel}");
        }

        // Second run: targets exist → absent-only → writes nothing.
        let second = materialize_contracts(&root, dir, false).unwrap();
        assert!(
            second.is_empty(),
            "materialize must be idempotent (absent-only)"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn materialize_force_refreshes_stale_seed() {
        // M2: a stale local seed (an old derived copy) must be refreshed by force=true,
        // while force=false stays absent-only and leaves an existing target untouched.
        let root = std::env::temp_dir().join(format!("lmd_seeds_force_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";

        // Seed once, then overwrite a target with stale content.
        materialize_contracts(&root, dir, false).unwrap();
        let stale = root.join(dir).join("plan-recipes.lmd.md");
        std::fs::write(&stale, "# stale derived copy\n").unwrap();

        // Absent-only refuses to refresh it.
        let noop = materialize_contracts(&root, dir, false).unwrap();
        assert!(noop.is_empty(), "force=false must stay absent-only");
        assert_eq!(
            std::fs::read_to_string(&stale).unwrap(),
            "# stale derived copy\n",
            "force=false must not clobber an existing target"
        );

        // force=true rewrites it back to the embedded seed content.
        let refreshed = materialize_contracts(&root, dir, true).unwrap();
        assert!(
            refreshed.iter().any(|p| p.ends_with("plan-recipes.lmd.md")),
            "force=true must (re)write plan-recipes"
        );
        assert_eq!(
            std::fs::read_to_string(&stale).unwrap(),
            PLAN_RECIPES,
            "force=true must refresh the stale seed to the embedded content"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_heals_a_stale_untouched_seed_silently() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_heal_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        // First refresh writes the lock for the pristine tree.
        refresh_contracts(&root, dir).unwrap();

        // Simulate "the embedded seed moved on": pin an OLD hash in the lock and put the
        // matching old content on disk. Local == lock → untouched → may heal.
        let target = root.join(dir).join("plan-recipes.lmd.md");
        let old = "# an older embedded copy\n";
        std::fs::write(&target, old).unwrap();
        let mut lock = crate::lock::Lock::load(&root);
        lock.set(
            "lean-md/plan-recipes.lmd.md",
            &crate::hashx::sha256_hex(old.as_bytes()),
        );
        lock.save(&root).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            PLAN_RECIPES,
            "stale + untouched must heal to the embedded seed"
        );
        assert!(
            report
                .healed
                .iter()
                .any(|p| p.ends_with("plan-recipes.lmd.md"))
        );
        assert!(report.preserved.is_empty(), "no .new for an untouched seed");
        assert!(
            !target.with_extension("md.new").exists(),
            "must not litter a .new"
        );
        // The lock followed along, so the next run is a no-op.
        assert!(refresh_contracts(&root, dir).unwrap().is_quiet());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_never_overwrites_a_user_edit_and_writes_new_beside_it() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_edit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap(); // lock now matches the pristine tree

        let target = root.join(dir).join("lang/rust.lmd.md");
        let edit = "# my project rule\n";
        std::fs::write(&target, edit).unwrap(); // local != lock → user edit

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            edit,
            "a user edit must NEVER be clobbered"
        );
        let new_file = root.join(dir).join("lang/rust.lmd.md.new");
        assert!(
            new_file.exists(),
            ".new must be written beside the edited seed"
        );
        assert!(report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")));
        assert!(report.healed.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_tree_without_a_lock_is_treated_conservatively() {
        // Today's state: 4 stale seeds, no lock, provenance unknown. We must not guess
        // "untouched" — that would clobber whatever the user did before locks existed.
        let root = std::env::temp_dir().join(format!("lmd_refresh_legacy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        let target = root.join(dir).join("tooling/mcp-tools.lmd.md");
        std::fs::write(&target, "# a pre-lock local copy\n").unwrap();
        assert!(!root.join(".lean-ctx/lean-md.lock").exists());

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# a pre-lock local copy\n",
            "unknown provenance must never be overwritten"
        );
        assert!(root.join(dir).join("tooling/mcp-tools.lmd.md.new").exists());
        assert!(
            report
                .preserved
                .iter()
                .any(|p| p.ends_with("mcp-tools.lmd.md"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_of_a_current_tree_is_a_silent_noop() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_noop_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            report.is_quiet(),
            "a current tree must produce no report at all"
        );
        assert!(report.healed.is_empty() && report.preserved.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_newly_registered_seed_materializes_without_a_new_file() {
        // task-5 adds two seeds AFTER locks exist in the field. An absent target is not a
        // user edit — it must just appear, silently.
        let root = std::env::temp_dir().join(format!("lmd_refresh_fresh_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();

        let (rel, _) = PROJECT_SEEDS[0];
        std::fs::remove_file(root.join(dir).join(rel)).unwrap();
        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            root.join(dir).join(rel).exists(),
            "absent seed must be (re)written"
        );
        assert!(
            report.preserved.is_empty(),
            "an absent target is not a user edit"
        );
        assert!(!root.join(dir).join(format!("{rel}.new")).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_recipes_import() {
        // @import plan-recipes + @call test(...) expands, and vars.toml overrides the
        // inline @var default (test_cmd). jail_root = a materialized seed tree.
        let root = std::env::temp_dir().join(format!("lmd_recipes_{}", std::process::id()));
        let vars_dir = root.join(".lean-ctx/lean-md");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&vars_dir).unwrap();
        materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        // PLAN_RECIPES is not yet a PROJECT_SEEDS entry (that wiring lands in
        // Subplan-4-Task-2), so stage it directly at the resolver's target path
        // for this test — same target Task-2's PROJECT_SEEDS entry will use.
        std::fs::write(vars_dir.join("plan-recipes.lmd.md"), PLAN_RECIPES).unwrap();
        std::fs::write(
            vars_dir.join("vars.toml"),
            "test_cmd = \"cargo nextest run\"\n",
        )
        .unwrap();

        let src = "\
@lean-md
consumer: ai

@var test_cmd default=\"cargo test\"
@import .lean-ctx/lean-md/plan-recipes /
@phase \"task-1\"
@call test(demo)
@phase-end
";
        let out =
            crate::skills::render_source_with_phase(src, Some("task-1"), None, None, root.clone())
                .unwrap();
        assert!(
            out.contains("cargo nextest run demo"),
            "recipe did not expand with vars override: {out}"
        );
        assert!(!out.contains("@call test"), "@call not expanded: {out}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_template_header_declares_crp_compact() {
        // Terseness rework: the template header binds crp: compact (drives the dispatch
        // CRP line + apply_crp_hook deterministically), alongside consumer: ai.
        assert!(
            PLAN_TEMPLATE.contains("consumer: ai"),
            "template header must keep consumer: ai"
        );
        assert!(
            PLAN_TEMPLATE.contains("crp: compact"),
            "template header must declare crp: compact"
        );
    }

    #[test]
    fn plan_template_meta_declares_lint_cmd() {
        // The meta-head declares lint_cmd once (pattern of test_cmd); vars.toml wins.
        assert!(
            PLAN_TEMPLATE.contains("@var lint_cmd"),
            "template meta-head must declare @var lint_cmd"
        );
    }

    #[test]
    fn plan_template_self_documents() {
        // Self-documenting: guidance markers present, no superpowers token.
        assert!(PLAN_TEMPLATE.contains("One @phase per task"));
        assert!(PLAN_TEMPLATE.contains("@call test"));
        assert!(PLAN_TEMPLATE.contains("anchor it"));
        assert!(!PLAN_TEMPLATE.to_lowercase().contains("superpowers"));

        // The real example task renders cleanly against a materialized seed tree.
        let root = std::env::temp_dir().join(format!("lmd_tmpl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        // plan-template's meta-head imports plan-recipes; same staging note as
        // plan_recipes_import above (PROJECT_SEEDS wiring lands in Task-2).
        std::fs::write(
            root.join(".lean-ctx/lean-md/plan-recipes.lmd.md"),
            PLAN_RECIPES,
        )
        .unwrap();

        let out = crate::skills::render_source_with_phase(
            PLAN_TEMPLATE,
            Some("task-1"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("foo_adds_one"),
            "example task did not render the test recipe: {out}"
        );
        assert!(
            out.contains("pub fn foo"),
            "new-code block missing from rendered task: {out}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_recipes_all_documented() {
        // Every @define's first non-empty body line is an HTML-comment description,
        // so the --signatures index (Subplan 1) carries a doc line for each macro.
        let lines: Vec<&str> = PLAN_RECIPES.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("@define ") {
                let doc = lines[i + 1..]
                    .iter()
                    .find(|l| !l.trim().is_empty())
                    .copied()
                    .unwrap_or("");
                assert!(
                    doc.trim_start().starts_with("<!--"),
                    "@define on line {} lacks a description comment: {line}",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn plan_recipes_carry_code_intel_macros() {
        // §4/§4a: the macro library must expose the code-intel recipes so plans can
        // @call them — presence is the enforced contract, not a suggestion.
        let defined = defined_macro_names(PLAN_RECIPES);
        for name in [
            "verify",
            "review_change",
            "check_smells",
            "inspect",
            "reformat_commit",
            "remember_decision",
            "recall_context",
            "callers",
        ] {
            assert!(
                defined.contains(name),
                "plan-recipes must define the {name} code-intel macro"
            );
        }
    }

    #[test]
    fn plan_recipes_carry_gate_and_render_check() {
        // Terseness rework: the recipe layer must expose the pre-commit gate and the
        // lmd render smoke so plans can @call them.
        let defined = defined_macro_names(PLAN_RECIPES);
        assert!(
            defined.contains("gate"),
            "plan-recipes must define the gate quality-bar recipe"
        );
        assert!(
            defined.contains("render_check"),
            "plan-recipes must define the render_check smoke recipe"
        );
    }

    #[test]
    fn no_orphan_call() {
        // Every @call NAME(...) starting a line in plan-template hits a @define NAME(...)
        // in plan-recipes (static check; runtime already surfaces `macro not found`).
        let defined = defined_macro_names(PLAN_RECIPES);
        assert!(defined.contains("test") && defined.contains("commit") && defined.contains("tdd"));

        for line in PLAN_TEMPLATE.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("@call ") {
                let name = rest.split('(').next().unwrap_or("").trim().to_string();
                assert!(
                    defined.contains(&name),
                    "@call {name} in plan-template has no matching @define in plan-recipes"
                );
            }
        }
    }

    #[test]
    fn plan_template_has_verify_and_close_contract() {
        // §6: every task ends with the fixed Verify & Close sequence; conditional
        // slots hang on observable predicates (refactor / multi-file / prior-task).
        assert!(
            PLAN_TEMPLATE.contains("Verify & Close"),
            "template must define the Verify & Close sequence"
        );
        for call in [
            "@call verify(",
            "@call gate(",
            "@call commit(",
            "@call remember_decision(",
        ] {
            assert!(
                PLAN_TEMPLATE.contains(call),
                "Verify & Close must include {call}"
            );
        }
        assert!(
            PLAN_TEMPLATE.contains("@call recall_context(")
                && PLAN_TEMPLATE.contains("@call callers(")
                && PLAN_TEMPLATE.contains("@call review_change("),
            "template must offer the conditional slots (recall/callers/review_change)"
        );
    }

    #[test]
    fn mcp_tools_is_a_usage_reference() {
        // §5a: tooling/mcp-tools is the directive USAGE reference for plan authors —
        // one line per woven directive: purpose · minimal form · when-to-use.
        let seed = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "tooling/mcp-tools.lmd.md")
            .map(|(_, c)| *c)
            .expect("mcp-tools seed must be registered");
        for directive in [
            "@refactor",
            "@review",
            "@smells",
            "@graph",
            "@impact",
            "@recall",
            "@remember",
        ] {
            assert!(
                seed.contains(directive),
                "mcp-tools usage reference must document {directive}"
            );
        }
        assert!(
            seed.contains("Use") || seed.contains("Nutze"),
            "usage reference must carry when-to-use guidance"
        );
    }

    #[test]
    fn rust_lang_pack_pins_refactor_rule() {
        // §7: Rust tasks with rename/move/extract must instruct @refactor
        // (ctx_refactor) — no hand-edits; @edit only for non-symbol changes.
        let seed = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "lang/rust.lmd.md")
            .map(|(_, c)| *c)
            .expect("rust lang seed must be registered");
        assert!(
            seed.contains("@refactor"),
            "rust pack must instruct @refactor"
        );
        assert!(
            seed.contains("rename") && seed.contains("extract"),
            "rust pack must name the refactor ops"
        );
        assert!(
            seed.contains("@edit"),
            "rust pack must scope @edit to non-symbol changes"
        );
    }
}
