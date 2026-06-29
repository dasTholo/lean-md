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

use harness::{Artifact, LMD_PHASES, collect_variant_b};
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
        .map(|x| (x.name.clone(), x.tokens_cl100k, x.tokens_o200k))
        .collect();
    let names_b: Vec<_> = b
        .iter()
        .map(|x| (x.name.clone(), x.tokens_cl100k, x.tokens_o200k))
        .collect();
    assert_eq!(names_a, names_b, "variant B must be byte-stable (#498)");
    let _ = std::fs::remove_dir_all(&dir);
}

use harness::{Metrics, TOOL_CALL_OVERHEAD_TOKENS, compute_metrics};

fn art(name: &str, t: usize) -> Artifact {
    Artifact {
        name: name.to_string(),
        tokens_cl100k: t,
        tokens_o200k: t,
    }
}

#[test]
fn compute_metrics_core_and_breakeven() {
    // A: SKILL.md=100, companion=50  → content 150
    let a = vec![art("SKILL.md", 100), art("testing-anti-patterns.md", 50)];
    // B: stub=10, 4 phases=20 each, companion=30
    let b = vec![
        art("SKILL.md (stub)", 10),
        art("phase:red", 20),
        art("phase:green", 20),
        art("phase:refactor", 20),
        art("phase:rationalizations", 20),
        art("companion:testing-anti-patterns", 30),
    ];

    let m: Metrics = compute_metrics(&a, &b);

    assert_eq!(m.a_content, 150);
    assert_eq!(m.a_with_overhead, 150 + TOOL_CALL_OVERHEAD_TOKENS); // 1 load
    // B full content = 10 + 80 + 30 = 120
    assert_eq!(m.b_content, 120);
    // B full overhead = 4 render calls (phases) + 1 companion render = 5
    assert_eq!(m.b_with_overhead, 120 + 5 * TOOL_CALL_OVERHEAD_TOKENS);

    // Cumulative (stub + k phases, companion excluded): k=1 → 10+20=30 content
    assert_eq!(
        m.b_cumulative[0],
        (1, 30, 30 + 1 * TOOL_CALL_OVERHEAD_TOKENS)
    );
    assert_eq!(
        m.b_cumulative[3],
        (4, 90, 90 + 4 * TOOL_CALL_OVERHEAD_TOKENS)
    );
}

use harness::format_summary;

#[test]
fn format_summary_is_deterministic_and_has_sections() {
    let a = vec![art("SKILL.md", 100), art("testing-anti-patterns.md", 50)];
    let b = vec![
        art("SKILL.md (stub)", 10),
        art("phase:red", 20),
        art("phase:green", 20),
        art("phase:refactor", 20),
        art("phase:rationalizations", 20),
        art("companion:testing-anti-patterns", 30),
    ];
    let m = compute_metrics(&a, &b);

    let s1 = format_summary(&a, &b, &m);
    let s2 = format_summary(&a, &b, &m);

    assert_eq!(s1, s2, "summary must be byte-stable (#498)");
    assert!(s1.contains("# Skill-Token-Vergleich"));
    assert!(s1.contains("cl100k_base"));
    assert!(s1.contains("o200k_base"));
    assert!(s1.contains("Break-even"));
    assert!(s1.contains("Reiner Inhalt"));
    // No timestamp/date marker in the body (#498).
    assert!(!s1.contains("Datum:"), "no date marker expected");
    assert!(
        !s1.contains("Generated"),
        "no generation timestamp expected"
    );
}
