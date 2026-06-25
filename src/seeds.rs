//! Project materialization of lang/tooling/.ext seeds (Spec §5.4, layer B).
//! Seeds are binary-embedded (`include_str!`); `materialize_contracts` copies
//! them into the project's `contracts_dir` ONLY when the target is absent, so a
//! user's edits are never clobbered. Resolution order: project file overrides
//! the embedded seed (handled by FragmentRegistry's jailed file fallback).

use std::path::{Path, PathBuf};

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
}
