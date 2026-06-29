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

/// Estimated extra tokens per ctx_md_render roundtrip (tool-use block:
/// name + {skill, phase} args + tool_result wrapper). Disclosed in SUMMARY.md
/// and tunable — this is a model assumption, not a measured constant.
pub const TOOL_CALL_OVERHEAD_TOKENS: usize = 40;

/// Aggregated A/B comparison (all token sums in cl100k).
#[derive(Debug, Clone)]
pub struct Metrics {
    pub a_content: usize,
    pub a_with_overhead: usize,
    pub b_content: usize,
    pub b_with_overhead: usize,
    /// (k, cumulative_content, cumulative_with_overhead) for k=1..=phase count.
    /// Stub always included; companion excluded (on-demand only).
    pub b_cumulative: Vec<(usize, usize, usize)>,
}

fn sum_cl100k(arts: &[Artifact]) -> usize {
    arts.iter().map(|a| a.tokens_cl100k).sum()
}

/// Compute the core metric + cumulative break-even table.
pub fn compute_metrics(variant_a: &[Artifact], variant_b: &[Artifact]) -> Metrics {
    let a_content = sum_cl100k(variant_a);
    let a_with_overhead = a_content + TOOL_CALL_OVERHEAD_TOKENS; // single skill load

    let stub = variant_b
        .iter()
        .find(|a| a.name == "SKILL.md (stub)")
        .map(|a| a.tokens_cl100k)
        .unwrap_or(0);
    let phases: Vec<usize> = variant_b
        .iter()
        .filter(|a| a.name.starts_with("phase:"))
        .map(|a| a.tokens_cl100k)
        .collect();
    let companion: usize = variant_b
        .iter()
        .filter(|a| a.name.starts_with("companion:"))
        .map(|a| a.tokens_cl100k)
        .sum();

    let b_content = stub + phases.iter().sum::<usize>() + companion;
    // Full-build overhead = one render call per phase + one per companion.
    let full_calls = phases.len()
        + variant_b
            .iter()
            .filter(|a| a.name.starts_with("companion:"))
            .count();
    let b_with_overhead = b_content + full_calls * TOOL_CALL_OVERHEAD_TOKENS;

    let mut b_cumulative = Vec::new();
    let mut running = stub;
    for (i, ph) in phases.iter().enumerate() {
        let k = i + 1;
        running += ph;
        b_cumulative.push((k, running, running + k * TOOL_CALL_OVERHEAD_TOKENS));
    }

    Metrics {
        a_content,
        a_with_overhead,
        b_content,
        b_with_overhead,
        b_cumulative,
    }
}

use std::fmt::Write as _;

/// Render the deterministic SUMMARY.md body (no timestamps, #498).
pub fn format_summary(variant_a: &[Artifact], variant_b: &[Artifact], metrics: &Metrics) -> String {
    let mut s = String::new();

    s.push_str("# Skill-Token-Vergleich — SUMMARY\n\n");
    s.push_str("Neutrales A/B: A = superpowers (Monolith), B = lmd (Phasen-Rendering).\n\n");

    s.push_str("## Annahmen\n\n");
    s.push_str("- Tokenizer: `cl100k_base` (primär, ~3% von Claudes echtem Tokenizer); ");
    s.push_str("`o200k_base` (Parität mit lean-ctx-Ledger).\n");
    let _ = writeln!(
        s,
        "- Tool-Call-Overhead pro `ctx_md_render`-Roundtrip: {} Tokens (Modellannahme, justierbar).\n",
        TOOL_CALL_OVERHEAD_TOKENS
    );

    s.push_str("## Artefakte (Tokens je Familie)\n\n");
    s.push_str("| Variante | Artefakt | cl100k | o200k |\n|---|---|---|---|\n");
    for a in variant_a {
        let _ = writeln!(
            s,
            "| A | {} | {} | {} |",
            a.name, a.tokens_cl100k, a.tokens_o200k
        );
    }
    for b in variant_b {
        let _ = writeln!(
            s,
            "| B | {} | {} | {} |",
            b.name, b.tokens_cl100k, b.tokens_o200k
        );
    }
    s.push('\n');

    s.push_str("## Kernmetrik (cl100k)\n\n");
    s.push_str("| Metrik | A (superpowers) | B (lmd, Vollausbau) | Δ (B−A) |\n|---|---|---|---|\n");
    let _ = writeln!(
        s,
        "| Reiner Inhalt | {} | {} | {} |",
        metrics.a_content,
        metrics.b_content,
        metrics.b_content as i64 - metrics.a_content as i64
    );
    let _ = writeln!(
        s,
        "| Inkl. Ablauf-Overhead | {} | {} | {} |",
        metrics.a_with_overhead,
        metrics.b_with_overhead,
        metrics.b_with_overhead as i64 - metrics.a_with_overhead as i64
    );
    s.push('\n');

    s.push_str("## Break-even (B kumulativ, Stub + k Phasen)\n\n");
    s.push_str("| k Phasen | B Inhalt | B inkl. Overhead | vs. A Inhalt | vs. A inkl. Overhead |\n|---|---|---|---|---|\n");
    for (k, content, with_oh) in &metrics.b_cumulative {
        let c_cmp = if *content <= metrics.a_content {
            "B billiger"
        } else {
            "B teurer"
        };
        let o_cmp = if *with_oh <= metrics.a_with_overhead {
            "B billiger"
        } else {
            "B teurer"
        };
        let _ = writeln!(s, "| {k} | {content} | {with_oh} | {c_cmp} | {o_cmp} |");
    }
    s.push('\n');

    s
}
