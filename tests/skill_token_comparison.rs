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
