# Skill Token Comparison — Benchmark

Neutral A/B: **A** = `superpowers/test-driven-development` (monolith),
**B** = `lmd-test-driven-development` (phased rendering).

## Layer A — deterministic trace (automated)

    cargo run --example skill-token-comparison

Renders variant B in-process, tokenizes both variants (tiktoken-rs:
`cl100k_base` primary, `o200k_base` parity) and writes `SUMMARY.md`
(byte-stable, #498). Test: `cargo nextest run -E 'binary(skill_token_comparison)'`.

The `TOOL_CALL_OVERHEAD_TOKENS` assumption (roundtrip overhead per
`ctx_md_render`) is named in `harness.rs` and disclosed in `SUMMARY.md` — tunable.

## Layer B — subagent validation (manual, mdai-adapted)

The same mini TDD task (one small function + one bugfix) is solved per variant,
once per pressure variant:

| Variant | Meaning |
|---|---|
| `cold`      | no constraint, free hand |
| `time`      | explicit time pressure in the prompt |
| `authority` | tech-lead override in the prompt ("just do it directly, no ceremony") |

Each subagent report records **verbatim**: which skill artifacts were actually
loaded (variant B: which phases were actually rendered) and how many tool calls
occurred. The loaded artifacts are re-counted with the same `harness` counting
→ real cumulative consumption.

Place reports under:

    variant-A-superpowers/<cold|time|authority>.md
    variant-B-lmd/<cold|time|authority>.md

Purpose: confirms/falsifies the Layer A hypothesis — do real agents stop at
RED/GREEN (so B never loads `refactor`/`rationalizations`), and is the tool-call
overhead smaller than the saved content tokens?
