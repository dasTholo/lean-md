@lean-md
consumer: ai

@phase "orient"

## Inline Plan Execution — Orient

**When to use:** a written `.lmd.md` implementation plan executed by the main agent in THIS
session — no per-task subagent; human review at batch checkpoints and a whole-branch final
gate. If there is no plan yet, use lmd-writing-plans first. Announce: "I'm using the
lmd-executing-plans skill to execute the plan."

**Ambient baseline (MANDATORY):** the plan deliberately omits ambient tool-discipline (it
assumes the executor already carries it). Inline the built-in hard-rules baseline so it holds
in this session too. (The subagent register/handoff contract `@dispatch` delivers does not
apply to inline execution — there is no per-task subagent.)

@include hard-rules

**Tool discipline:** all I/O + coordination runs through lean-ctx — no SDD bash scripts, no
scratch-ledger files. Progress → `ctx_session`; durable facts → `ctx_knowledge`; register →
`ctx_agent`.

**Read the plan critically:** get its structure via `lean-md render <plan> --list-phases` —
NEVER `ctx_read` the plan for the brief (a raw read returns the whole document's source — all
phases, unexpanded — not the isolated task brief). Bundle any
concerns about the plan into ONE question BEFORE Task 1, never mid-run.

**Isolation:** work on a dedicated feature branch, never `main`/`master` without explicit
human consent. Optionally take a shadow-git net before Task 1: `@call snapshot("pre-execution")`.

**Resume:** on start, `ctx_session load` + `ctx_knowledge recall` — the MCP server auto-injects
the ACTIVE SESSION block. Tasks already complete are NOT re-run (recovery map = `ctx_knowledge`
+ `git log`).

Once: `ctx_agent action=register agent_type=claude role=executor`.

next: render phase "preflight".
@phase-end

@phase "preflight"

## Preflight — enumerate tasks & set batch boundaries (once, before Task 1)

Enumerate the plan's tasks without pulling its content into context:
Run `lean-md render <plan> --list-phases` → an ordered `name<TAB>title` index (import-independent;
renders no bodies). Create one todo per task.

**Set batch boundaries:** decide the checkpoint points where the human reviews (e.g. after each
plan phase-group / before invasive tasks). Boundaries are executor's judgment, fixed once here
and noted in `ctx_session`.

**Pre-flight conflict scan:** check the plan for internal contradictions or conflicts with the
current tree; bundle ALL concerns into ONE question to the human — before Task 1, never mid-run.

next: render phase "execute".
@phase-end

@phase "execute"

## Execute — per-task loop, inline (main agent, no subagent)

For each task in order, until the next batch boundary or BLOCKED:

1. At batch start record BASE: `BASE = @query "git rev-parse HEAD"` (local git → `@query`, NOT
   `ctx_git_read`). Fidelity-critical; never `HEAD~1` (drops multi-commit tasks).
2. Task brief = the phase render, captured raw:
   `ctx_shell(command="cargo run -q --bin lean-md -- render <plan> --phase task-N --consumer=ai", raw=true)`
   — raw is mandatory (double-compression would mangle the code to write). Warm the task's
   source files in ONE `ctx_multi_read paths=[…]`.
3. Todo `in_progress`.
4. Snapshot around the edit: `@call snapshot("pre-task-N")` before, `@call snapshot("post-task-N")`
   after — captures exactly what changed.
5. Follow the steps EXACTLY (the plan is bite-sized); run the plan-specified verification —
   typically `@call gate(<paths>)` (reformat + lint + full test suite) and `@read mode=diff`
   instead of copy-paste inspection. Apply reference skills when the plan says so.
6. Todo `completed`; `ctx_session action=task "Task N [x%]"`; durable gotchas → `ctx_knowledge`.

Continuous execution (≤1 line of narration between tool calls); pause only at a batch boundary,
on BLOCKED, or on genuine ambiguity.

**Stop-and-Ask (STOP immediately on):** a blocker (missing dependency, failing test, unclear
instruction); a critical plan gap before start; repeated verification failure. → Ask, don't
guess. **Revisit** (back to `orient`/plan review): the human updates the plan on feedback, or
the fundamental approach must be rethought — never force a blocker through.

next: render phase "checkpoint".
@phase-end

@phase "checkpoint"

## Checkpoint — human review at a batch boundary

- `HEAD = @query "git rev-parse HEAD"`.
- Present the diff to the human: `@query "git diff BASE..HEAD"` + `@read mode=diff` — the human
  is the reviewer (inline, no reviewer subagent).
- `@call compress()` — context checkpoint of the long session.
- Wait for approval. Approved → next batch (back to `execute`). Changes wanted → fix inline,
  revisit if needed. Fundamental approach wrong → back to `orient`/plan review.

next: render phase "final-gate".
@phase-end

@phase "final-gate"

## Final Gate — whole-branch review (after the last task)

- Deterministic pre-pass: `@query "git diff merge-base..HEAD" | @review diff-review` plus
  `@smells` (scan the changed surface).
- Present findings + diff to the human (inline, no reviewer subagent — a deliberate design
  choice for the inline-execution variant).
- Fix confirmed findings inline (one pass, re-run tests through the gate).

next: render phase "finish".
@phase-end

@phase "finish"

## Finish — branch completion

- Branch finishing via the external finishing-a-development-branch reference (merge / PR /
  cleanup choice presented to the human) — until an lmd port exists.
- Record the closing state via `ctx_session action=status`. No "next" render.

This is the terminal phase.
@phase-end
