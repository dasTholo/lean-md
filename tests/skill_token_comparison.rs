//! Integration test for the skill-token-comparison harness.
//! Binds the example's harness module via #[path] so nextest drives it
//! without pulling tiktoken-rs into the lean_md lib.

#[path = "../benchmarks/skill-token-comparison/harness.rs"]
mod harness;

use harness::{Family, token_count};

#[test]
fn token_count_empty_is_zero() {
    assert_eq!(token_count("", Family::Cl100k), 0);
    assert_eq!(token_count("", Family::O200k), 0);
}

#[test]
fn token_count_known_short_string() {
    // "hello world" = 2 BPE tokens in both cl100k_base and o200k_base.
    assert_eq!(token_count("hello world", Family::Cl100k), 2);
    assert_eq!(token_count("hello world", Family::O200k), 2);
}

#[test]
fn token_count_is_deterministic() {
    let text = "fn main() { println!(\"hi\"); }";
    assert_eq!(
        token_count(text, Family::Cl100k),
        token_count(text, Family::Cl100k)
    );
}

use harness::{Artifact, collect_variant_a};
use std::fs;

fn write_fixture(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    fs::write(&p, body).unwrap();
    p
}

#[test]
fn collect_variant_a_tokenizes_present_files() {
    let dir = std::env::temp_dir().join(format!("stc_a_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let skill = write_fixture(&dir, "SKILL.md", "# TDD\nWrite the test first.\n");
    let comp = write_fixture(
        &dir,
        "testing-anti-patterns.md",
        "# Anti-Patterns\nNo mocks.\n",
    );

    let arts = collect_variant_a(&skill, &comp);

    assert_eq!(arts.len(), 2);
    assert_eq!(arts[0].name, "SKILL.md");
    assert!(arts[0].tokens_cl100k > 0);
    assert!(arts[0].tokens_o200k > 0);
    assert_eq!(arts[1].name, "testing-anti-patterns.md");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn collect_variant_a_skips_missing_companion() {
    let dir = std::env::temp_dir().join(format!("stc_a_miss_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let skill = write_fixture(&dir, "SKILL.md", "# TDD\n");
    let missing = dir.join("testing-anti-patterns.md");

    let arts = collect_variant_a(&skill, &missing);

    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0].name, "SKILL.md");
    let _ = fs::remove_dir_all(&dir);
}
