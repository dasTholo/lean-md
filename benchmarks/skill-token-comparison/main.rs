//! Skill token-comparison benchmark (Schicht A). Renders both TDD skill
//! variants in-process, tokenizes the artifacts, writes SUMMARY.md.
//! tiktoken-rs is a dev-dependency; this code lives ONLY in the example
//! target (never bundled into the lean_md lib/bin). No #[cfg] gate needed.

#[path = "harness.rs"]
mod harness;

fn main() {
    // Wired up in Task 6.
}
