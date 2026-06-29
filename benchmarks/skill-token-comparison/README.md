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
`writing-skills/testing-skills-with-subagents.md` (worked example:
`writing-skills/examples/CLAUDE_MD_TESTING.md`): **testing a skill is just TDD
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

### Variants compared (incl. the NULL floor)

| Variant | Delivery | Skill tokens loaded |
|---|---|---|
| NULL | no skill at all | 0 — the floor |
| A — superpowers | one monolithic `SKILL.md` (+ companion) loaded up front | full monolith |
| B — lmd | phases rendered on demand, one at a time | only the phases actually reached |

The NULL baseline is required by the source methodology: *if you didn't watch an
agent work without the skill, you don't know what the skill changes.* It also
fixes the lower bound — every token a variant spends above NULL must be repaid by
better behavior.

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

### Running Layer B (four steps per scenario)

Run each `scenarios/*.md` through four steps, recording verbatim at each:

1. **NULL baseline** — no skill loaded. Record the agent's natural choice (A/B/C)
   and its rationalizations. Skill tokens spent: 0.
2. **Run the variant** — once for A, once for B. Record which artifacts were
   actually loaded (variant B: which phases were rendered, plus the companion if
   pulled) and the tool-call count.
3. **Pressure test** — the scenario already stacks 3+ pressures; check whether the
   agent still completes the cycle or stops early at RED/GREEN (so B never loads
   `refactor` / `rationalizations`).
4. **Meta-test** — ask the agent afterward: "you had the skill and stopped at X —
   why?" Capture the answer; it explains the early-stop the token numbers show.

Re-count the loaded artifacts with the same `harness` counting → real cumulative
consumption per variant, measured against the NULL floor.

File reports under:

    variant-NULL/<scenario-id>.md
    variant-A-superpowers/<scenario-id>.md
    variant-B-lmd/<scenario-id>.md

Purpose: confirm or falsify the Layer A hypothesis with real agent behavior —
do agents under stacked pressure stop at RED/GREEN (so B never loads
`refactor` / `rationalizations`), and is the tool-call overhead smaller than the
content tokens B saves?

> Background on why stacked pressure raises compliance pressure (authority,
> scarcity, commitment): see `writing-skills/persuasion-principles.md` in the
> superpowers distribution.
