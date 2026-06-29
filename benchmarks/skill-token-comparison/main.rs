//! Skill token-comparison benchmark (Schicht A). Renders both TDD skill
//! variants in-process, tokenizes the artifacts, writes SUMMARY.md.
//! tiktoken-rs is a dev-dependency; this code lives ONLY in the example
//! target (never bundled into the lean_md lib/bin). No #[cfg] gate needed.

#[path = "harness.rs"]
mod harness;

use std::path::{Path, PathBuf};

const SP_SKILL: &str = "/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/test-driven-development/SKILL.md";
const SP_COMPANION: &str = "/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/test-driven-development/testing-anti-patterns.md";

fn main() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stub = repo_root.join("content/skills/lmd-test-driven-development/SKILL.md");
    let out = repo_root.join("benchmarks/skill-token-comparison/SUMMARY.md");

    let variant_a = harness::collect_variant_a(Path::new(SP_SKILL), Path::new(SP_COMPANION));
    let variant_b = harness::collect_variant_b(&stub, repo_root.clone());
    let metrics = harness::compute_metrics(&variant_a, &variant_b);
    let summary = harness::format_summary(&variant_a, &variant_b, &metrics);

    std::fs::write(&out, summary).expect("write SUMMARY.md");
    println!("wrote {}", out.display());
}
