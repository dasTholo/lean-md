//! Core logic for the skill-token-comparison benchmark.
//! Shared by main.rs (example target) and tests/skill_token_comparison.rs
//! (integration test) via #[path]. Uses tiktoken-rs (dev-dep) + lean_md lib.

/// Tokenizer family. cl100k_base ~ Claude (~3% of actual); o200k_base matches
/// the lean-ctx savings ledger (COUNTING_FAMILY) for cross-comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Cl100k,
    O200k,
}

/// Count BPE tokens of `text` under `family`. Empty text → 0.
pub fn token_count(text: &str, family: Family) -> usize {
    if text.is_empty() {
        return 0;
    }
    let bpe = match family {
        Family::Cl100k => tiktoken_rs::cl100k_base(),
        Family::O200k => tiktoken_rs::o200k_base(),
    }
    .expect("tiktoken base tables load");
    bpe.encode_with_special_tokens(text).len()
}

use std::path::Path;

/// One tokenized skill artifact, counted under both families.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub name: String,
    pub tokens_cl100k: usize,
    pub tokens_o200k: usize,
}

impl Artifact {
    fn from_text(name: &str, text: &str) -> Self {
        Artifact {
            name: name.to_string(),
            tokens_cl100k: token_count(text, Family::Cl100k),
            tokens_o200k: token_count(text, Family::O200k),
        }
    }
}

fn artifact_from_file(path: &Path) -> Option<Artifact> {
    let text = std::fs::read_to_string(path).ok()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    Some(Artifact::from_text(&name, &text))
}

/// Variant A (superpowers monolith): the full SKILL.md plus its companion.
/// Missing files are skipped (no panic).
pub fn collect_variant_a(skill_md: &Path, companion_md: &Path) -> Vec<Artifact> {
    [skill_md, companion_md]
        .into_iter()
        .filter_map(artifact_from_file)
        .collect()
}

use lean_md::header::Consumer;
use lean_md::skills::{render_companion, render_skill};
use std::path::PathBuf;

const LMD_SKILL: &str = "lmd-test-driven-development";
const LMD_COMPANION: &str = "testing-anti-patterns";

/// The lmd TDD phase sequence, rendered on demand one at a time.
pub const LMD_PHASES: [&str; 4] = ["red", "green", "refactor", "rationalizations"];

/// Variant B (lmd phased rendering): stub SKILL.md + each rendered phase +
/// the rendered companion. Rendering is in-process (Consumer::Ai, crp=None).
pub fn collect_variant_b(stub_md: &Path, jail_root: PathBuf) -> Vec<Artifact> {
    let mut arts = Vec::new();

    if let Ok(text) = std::fs::read_to_string(stub_md) {
        arts.push(Artifact::from_text("SKILL.md (stub)", &text));
    }

    for phase in LMD_PHASES {
        let rendered = render_skill(
            LMD_SKILL,
            Some(phase),
            Some(Consumer::Ai),
            None,
            jail_root.clone(),
        )
        .unwrap_or_else(|e| panic!("render phase {phase}: {e}"));
        arts.push(Artifact::from_text(&format!("phase:{phase}"), &rendered));
    }

    let companion = render_companion(
        LMD_SKILL,
        LMD_COMPANION,
        Some(Consumer::Ai),
        None,
        jail_root,
    )
    .unwrap_or_else(|e| panic!("render companion: {e}"));
    arts.push(Artifact::from_text(
        &format!("companion:{LMD_COMPANION}"),
        &companion,
    ));

    arts
}
