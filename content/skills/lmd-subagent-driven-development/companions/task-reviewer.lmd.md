# Task Reviewer (lmd companion — two-verdict per-task review brief)

Dispatched per task via `@dispatch skill="lmd-subagent-driven-development"
companion="task-reviewer" role=review`; the dispatch contract is auto-prepended.

You review ONE task's implementation. Read the task brief (its phase render) + the implementer's
`ctx_agent` post, then fetch the diff YOURSELF (`@read mode=diff` over BASE..HEAD). **Do not
trust the report** — verify every claim against the diff.

Produce TWO verdicts from this one diff read.

## Part 1 — Spec Compliance
- Missing: brief requirements not implemented.
- Extra: code beyond the brief (scope creep).
- Misunderstood: implemented, but not what the brief meant.
- ⚠️: things you could not verify from the diff alone (the controller resolves these).

## Part 2 — Code Quality
- Correctness, error handling, test quality (does the test actually exercise the behavior?),
  naming, dead code, adherence to the plan's Global Constraints (passed to you verbatim).

## Calibration
Rate each finding Critical / Important / Minor. `plan-mandated` = the plan explicitly required
it → do NOT flag as a defect; if you disagree with the plan, surface it as a plan-conflict for
the human, do not silently drop it.

## Output
Return both verdicts + an overall ✅ (Approved) / ❌ (Needs-fixes) / ⚠️ (open items). Post
findings via `ctx_agent action=post category=finding`; return the verdict summary to the
controller.
