//! Project materialization of lang/tooling/.ext seeds (Spec §5.4, layer B).
//! Seeds are binary-embedded (`include_str!`); `materialize_contracts` copies
//! them into the project's `contracts_dir` ONLY when the target is absent, so a
//! user's edits are never clobbered. Resolution order: project file overrides
//! the embedded seed (handled by FragmentRegistry's jailed file fallback).

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
];

/// Materialize embedded project seeds into `<project_root>/<contracts_dir>`.
/// Absent-only (idempotent); returns the paths actually written.
pub fn materialize_contracts(
    project_root: &Path,
    contracts_dir: &str,
) -> std::io::Result<Vec<PathBuf>> {
    let base = project_root.join(contracts_dir);
    let mut written = Vec::new();
    for (rel, content) in PROJECT_SEEDS {
        let target = base.join(rel);
        if target.exists() {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn materialize_writes_then_is_idempotent() {
        let root = std::env::temp_dir().join(format!("lmd_seeds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";

        let first = materialize_contracts(&root, dir).unwrap();
        assert_eq!(
            first.len(),
            PROJECT_SEEDS.len(),
            "first run writes all seeds"
        );
        for (rel, _) in PROJECT_SEEDS {
            assert!(root.join(dir).join(rel).exists(), "seed not written: {rel}");
        }

        // Second run: targets exist → absent-only → writes nothing.
        let second = materialize_contracts(&root, dir).unwrap();
        assert!(
            second.is_empty(),
            "materialize must be idempotent (absent-only)"
        );

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
        materialize_contracts(&root, ".lean-ctx/lean-md").unwrap();
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
        materialize_contracts(&root, ".lean-ctx/lean-md").unwrap();
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
}
