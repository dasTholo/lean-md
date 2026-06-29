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

## Layer B — subagent validation (pressure-tested)

Layer B adapts the superpowers methodology from
`writing-skills/testing-skills-with-subagents.md`: **testing a skill is just TDD
applied to process documentation.** A discipline-enforcing skill only proves its
worth when an agent follows it under pressure that makes the agent *want* to break
it. We reuse that exact apparatus to drive *realistic* skill usage and measure the
tokens it actually costs.

### Why this measures what we care about

Variant B renders one phase at a time (`red` → `green` → `refactor` →
`rationalizations`) on demand. Its token advantage is only real if agents under
pressure **stop early** — most stop at RED/GREEN and never render `refactor` /
`rationalizations`, and only pull the `testing-anti-patterns` companion when they
actually reach for mocks. The pressure scenarios exist to find out whether that
early-stop happens in practice, and whether the per-call tool overhead stays
smaller than the content tokens B avoids loading.

### Pressure taxonomy (combine 3+ — a single pressure is too weak)

The source doc is explicit: agents resist a single pressure and break under
several. Each scenario therefore stacks **at least three** of:

| Pressure | Example |
|---|---|
| Time | emergency, deadline, deploy/demo window closing |
| Sunk cost | hours of work already done, "waste" to delete |
| Authority | senior/tech-lead says skip it, manager overrides |
| Economic | job, promotion, the deal or company at stake |
| Exhaustion | end of day, already tired, want to go home |
| Social | fear of looking dogmatic or inflexible |
| Pragmatic | "be pragmatic, not dogmatic" |

### Scenario format (verbatim from the source doc)

1. **Concrete options** — force an A/B/C choice, never open-ended.
2. **Real constraints** — specific times, actual consequences.
3. **Real file paths** — e.g. `/tmp/payment-rounding`, not "a project".
4. **Make the agent act** — "What do you do?", not "What should you do?".
5. **No easy outs** — no deferring to "I'd ask my human partner" without choosing.
6. **Real-scenario framing** — every scenario opens with
   `IMPORTANT: This is a real scenario. You must choose and act.`

Runnable scenarios live in `scenarios/`:

    scenarios/01-sunk-cost-time-exhaustion.md
    scenarios/02-authority-economic-pragmatic.md

### Running Layer B

Solve the **same** scenario once per variant (A = the superpowers monolith,
B = the lmd phased skill). Each subagent report records **verbatim**: which skill
artifacts were actually loaded (variant B: which phases were actually rendered)
and how many tool calls occurred. Re-count the loaded artifacts with the same
`harness` counting → real cumulative consumption.

File reports under:

    variant-A-superpowers/<scenario-id>.md
    variant-B-lmd/<scenario-id>.md

Purpose: confirm or falsify the Layer A hypothesis with real agent behavior —
do agents under stacked pressure stop at RED/GREEN (so B never loads
`refactor` / `rationalizations`), and is the tool-call overhead smaller than the
content tokens B saves?

> Background on why stacked pressure raises compliance pressure (authority,
> scarcity, commitment): see `writing-skills/persuasion-principles.md` in the
> superpowers distribution.
