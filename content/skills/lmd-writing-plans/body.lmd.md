@lean-md
consumer: ai

@phase "pre-context"

## Writing Implementation Plans

Write comprehensive implementation plans assuming the engineer has zero context
for our codebase and questionable taste. Document what they need: which files to
touch for each task, code, testing, docs they might need to check, how to test it.
Give them the whole plan as bite-sized tasks. DRY. YAGNI. TDD. Frequent commits.

Assume a skilled developer who knows almost nothing about our toolset or problem
domain, and doesn't know good test design very well.

**Announce at start:** "I'm using the lmd-writing-plans skill to create the
implementation plan."

**Tool discipline (reference, not a gate):** all I/O and code-intel run through
lean-ctx tools — the directive **usage reference** (purpose · minimal form ·
when-to-use) is `tooling/mcp-tools`. Language-specific symbol/edit/reformat
conventions live in the project's lang-pack — detect the project language from its
manifest/extensions (via `@list`/`@search`) and reference the matching pack; in a
Rust project that is `lang/rust`. The plan directive vocabulary is in
`gloss/directives`.

## Scope Check

If the spec covers multiple independent subsystems, it should have been broken into
sub-project specs during brainstorming. If it wasn't, suggest breaking this into
separate plans — one per subsystem. Each plan should produce working, testable
software on its own.

next: render phase "file-structure".
@phase-end

@phase "file-structure"

## File Structure

Before defining tasks, map out which files will be created or modified and what
each one is responsible for. This is where decomposition decisions get locked in.

- Design units with clear boundaries and well-defined interfaces. Each file should
  have one clear responsibility.
- You reason best about code you can hold in context at once, and your edits are
  more reliable when files are focused. Prefer smaller, focused files over large
  ones that do too much.
- Files that change together should live together. Split by responsibility, not by
  technical layer.
- In existing codebases, follow established patterns. If the codebase uses large
  files, don't unilaterally restructure — but if a file you're modifying has grown
  unwieldy, including a split in the plan is reasonable.

## Authoring with code-intel (measure before you decompose)

Before drawing task boundaries, measure the real dependency reach — don't guess:

- `@graph <callers|callees> <symbol>` / `@impact <symbol>` — the real dependency
  reach of a symbol (ctx_callgraph / ctx_impact). Justifies how invasive a task is.
- `@find <intent>` — locate the code a task anchors to, semantically
  (ctx_semantic_search).
- `@recall <query>` — **supplementary** (not mandatory): pull design decisions the
  brainstorm phase saved via `@remember` (ctx_knowledge recall). Useful mainly
  cross-session — carries rejected alternatives that never made it into the spec
  prose. The **spec file stays the primary source**; an empty knowledge store just
  means you read the spec.

This structure informs the task decomposition. Each task should produce
self-contained changes that make sense independently.

next: render phase "task-sizing".
@phase-end

@phase "task-sizing"

## Task Right-Sizing

A task is the smallest unit that carries its own test cycle and is worth a fresh
reviewer's gate. When drawing task boundaries: fold setup, configuration,
scaffolding, and documentation steps into the task whose deliverable needs them;
split only where a reviewer could meaningfully reject one task while approving its
neighbor. Each task ends with an independently testable deliverable.

## Bite-Sized Task Granularity

Each step is one concrete action (2-5 minutes). The standard TDD → gate → commit
cycle is boilerplate — express it as recipe `@call`s, not five spelled-out prose
steps, so the plan stays terse and the executor still gets the full text on render:

- `@call tdd(<case>)` — one red-to-green cycle (failing test, run red, implement,
  run green).
- `@call gate(<paths>)` — the pre-commit quality bar (reformat, lint, full test suite).
- `@call commit(<paths>, "<message>")` — stage and commit.

Spell out only what a recipe cannot carry: the actual test code, the new production
code, the exact interfaces, and any task-specific "Expected:" checks.

next: render phase "plan-format".
@phase-end

@phase "plan-format"

## Plan Format — write the plan as a `.lmd.md` document

A generated plan is a `.lmd.md` rendered task-on-demand. The executing controller
renders only the current task: `lean-md render <plan.lmd.md> --phase task-N` —
phase-isolation delivers exactly that task block (no cross-task leak). Copy the
shape from `.lean-ctx/lean-md/plan-template.lmd.md` and the macro library from
`.lean-ctx/lean-md/plan-recipes.lmd.md`.

**Meta-head (body-top, outside the phases):** Goal / Architecture / Global
Constraints — once, `@include`-referenceable — plus `@var` declarations (e.g.
`test_cmd`) and `@import .lean-ctx/lean-md/plan-recipes /` for the macro
vocabulary. The Global Constraints block carries the spec's project-wide
requirements verbatim; every task implicitly includes it.

**One `@phase "task-N"` per task.**

**No-loss rule inside a task (binding):**
- **existing** code → anchor it (`@symbol name` / `@read path mode=signatures` /
  `path:line`), resolved just-in-time from the warm cache — do NOT duplicate it
  verbatim.
- **new** code (does not exist yet) → verbatim, exactly as a context-free plan
  would.
- Interfaces / Consumes-Produces / commands / "Expected:" → verbatim, strict
  (No-Placeholders holds for intent and interfaces).
- Verification → `@read mode=diff` instead of copy-paste inspection.

**Terseness — the plan-template header carries `crp: compact`** (alongside
`consumer: ai`), and every task obeys two output rules:

- **output_rule #1 (no-loss):** new code verbatim; interfaces / commands / "Expected:"
  verbatim; existing code anchored — never dropped.
- **output_rule #2 (avoid repeating ambient context):** a plan never restates context
  the executor already carries. Repo/build plumbing (`include_str!` seed sync, manifest
  layout) appears at most ONCE as a Meta-head Architecture anchor, never per task;
  toolset rationale the dispatch contract re-supplies (Iron-Law quotes, spec-§
  cross-refs, determinism/#498 reminders) is omitted entirely. This rule touches ONLY
  ambient context — Intent, Interfaces/Consumes-Produces, NEW code, Commands and
  "Expected:" always stay verbatim.

**Future constraint:** once `lmd-executing-plans` is ported it MUST prepend the same
dispatch baseline — otherwise the omission leaks during inline execution.

**Boilerplate** (TDD cycle, commit, test-run) → `@call <recipe>(...)`; it expands
to full text at render time, so the executor loses nothing. To discover which
recipes exist, read the macro API index instead of the whole library:
`lean-md render .lean-ctx/lean-md/plan-recipes.lmd.md --signatures`.

**Reading the `.lmd.md` sources while authoring:** a plan, template, recipe
library or seed is an lmd source. `ctx_read` returns it **raw** (verbatim
directives) — read it directly for edit anchors. Rendering is explicit:
- recipe macro API (`plan-recipes.lmd.md`)              -> `render … --signatures`
- an existing plan / template phase brief               -> `render … --phase <p>`
- gateway-independent raw dump of a seed you must EDIT   -> `lean-md source <file>`

**Verification uses recipes, not copy-paste.** Inspect a change with
`@call verify("path/to/file")` (unified diff). For a public-API or multi-file
change, add the post-change gate `@call review_change()`.

## No Placeholders

Every step must contain the actual content an engineer needs. Never write: "TBD" /
"TODO" / "implement later"; "add appropriate error handling / validation / edge
cases"; "write tests for the above" without the test code; "similar to Task N"
without repeating the reference; steps that describe what to do without showing
how; references to types/functions/methods not defined in any task. For **existing**
code an anchor is not a placeholder — it resolves to real code on render; for
**new** code, show the code.

## Remember

- Exact file paths always.
- Complete content in every step — new code shown verbatim, existing code anchored.
- Exact commands with expected output.
- DRY, YAGNI, TDD, frequent commits.

next: render phase "write-plan".
@phase-end

@phase "write-plan"

## Writing the plan

Save plans to `docs/lean-md/plans/YYYY-MM-DD-<feature-name>.md` (user preferences
for plan location override this default).

If the spec covers multiple independent subsystems, write one plan per subsystem —
each producing working, testable software on its own — plus a short index plan that
states the decomposition, ordering and dependencies.

Persist plan state through the lean-ctx runtime only — **never** a `scratchpad/…`,
`/tmp/…` or git-ignored ledger file. Task progress and intermediate state ->
`ctx_session` (`action=task|finding|decision|status`); durable decisions/facts/
gotchas -> `ctx_knowledge` (`action=remember`); multi-agent coordination ->
`ctx_agent`. Then commit the plan document(s). Writing plan/task state to a scratch
file is a contract violation (see `CLAUDE.md` "No Brief-/Report-Files").

next: render phase "self-review".
@phase-end

@phase "self-review"

## Self-Review

After writing the complete plan, look at the spec with fresh eyes and check the
plan against it. This is a checklist you run yourself.

1. **Spec coverage:** Skim each section/requirement in the spec. Can you point to a
   task that implements it? List any gaps.
2. **Placeholder scan:** Search your plan for the red flags from "No Placeholders".
   Fix them. An anchor to existing code is NOT a placeholder.
3. **Type consistency:** Do the types, method signatures and property names in
   later tasks match what earlier tasks defined? A function called `clearLayers()`
   in Task 3 but `clearFullLayers()` in Task 7 is a bug.
4. **Ambient-context scan:** ambient context (repo/build plumbing, toolset rationale)
   appears once in the Meta-head, not repeated per task. Move any per-task restatement
   up into the Meta-head Architecture / Global Constraints block.

If you find issues, fix them inline. If you find a spec requirement with no task,
add the task.

Persist any gaps/findings from this pass through the lean-ctx runtime only —
**never** a scratch/ledger file: `ctx_session` (`action=finding`) for what this
review surfaces.

For an independent second pass, dispatch the plan-reviewer subagent (its brief is
the reviewer companion; the dispatch contract is auto-prepended):

@dispatch skill="lmd-writing-plans" companion="plan-reviewer" role=review to_agent="{{ controller_id }}"

The reviewer checks `PLAN_FILE_PATH` against `SPEC_FILE_PATH`, posts findings, and
returns a status (`Approved | Issues Found`).

next: render phase "handoff".
@phase-end

@phase "handoff"

## Execution Handoff

After saving the plan, offer the execution choice:

> "Plan complete and saved to `docs/lean-md/plans/<filename>.lmd.md`. Two execution
> options:
>
> **1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review
> between tasks, fast iteration.
>
> **2. Inline Execution** — execute tasks in this session with checkpoints for
> review.
>
> Which approach?"

- **If Subagent-Driven:** use the lmd-subagent-driven-development skill — fresh
  subagent per task + two-stage review.
- **If Inline Execution:** use the lmd-executing-plans skill — batch execution with
  checkpoints.

Record the execution handoff through the lean-ctx runtime only — **never** a
report/brief file: `ctx_session` (`action=status`) captures the chosen execution
mode and plan pointer for whoever picks the plan up.

This is the terminal phase — there is no "next" render.
@phase-end
