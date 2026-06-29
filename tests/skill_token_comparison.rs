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

use harness::collect_variant_a;
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

use harness::{LMD_PHASES, collect_variant_b};
use std::path::PathBuf;

#[test]
fn collect_variant_b_renders_phases_and_companion() {
    // Stub file optional; use the real repo stub if present, else a temp one.
    let dir = std::env::temp_dir().join(format!("stc_b_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stub = dir.join("SKILL.md");
    std::fs::write(&stub, "# stub\n").unwrap();

    let arts = collect_variant_b(&stub, PathBuf::from("."));

    // 1 stub + 4 phases + 1 companion = 6.
    assert_eq!(arts.len(), 6);
    for p in LMD_PHASES {
        let name = format!("phase:{p}");
        let a = arts
            .iter()
            .find(|a| a.name == name)
            .expect("phase artifact");
        assert!(a.tokens_cl100k > 0, "phase {p} rendered empty");
    }
    let comp = arts
        .iter()
        .find(|a| a.name == "companion:testing-anti-patterns")
        .expect("companion artifact");
    assert!(comp.tokens_cl100k > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn collect_variant_b_is_deterministic() {
    let dir = std::env::temp_dir().join(format!("stc_b_det_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stub = dir.join("SKILL.md");
    std::fs::write(&stub, "# stub\n").unwrap();

    let a = collect_variant_b(&stub, PathBuf::from("."));
    let b = collect_variant_b(&stub, PathBuf::from("."));
    let names_a: Vec<_> = a
        .iter()
        .map(|x| (x.name.clone(), x.tokens_cl100k))
        .collect();
    let names_b: Vec<_> = b
        .iter()
        .map(|x| (x.name.clone(), x.tokens_cl100k))
        .collect();
    assert_eq!(names_a, names_b, "variant B must be byte-stable (#498)");
    let _ = std::fs::remove_dir_all(&dir);
}
