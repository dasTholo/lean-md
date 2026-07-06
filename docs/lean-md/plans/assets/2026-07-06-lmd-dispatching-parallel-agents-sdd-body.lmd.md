@lean-md
consumer: ai

@phase "orient"

## Subagent-Driven Plan Execution — Orient

**When to use:** a written `.lmd.md` implementation plan with independent tasks, executed
in THIS session — one fresh implementer subagent per task, reviewed between tasks. If there
is no plan yet, use lmd-writing-plans first. Announce: "I'm using the
lmd-subagent-driven-development skill to execute the plan."

**Tool discipline:** all coordination runs through lean-ctx memory/coordination tools — no
external SDD bash scripts, no scratch-ledger files. Progress → `ctx_session`; durable
facts → `ctx_knowledge`; briefs/reports/batons → `ctx_agent`.

**Isolation:** work on a dedicated feature branch, never `main`/`master`. Optionally take a
shadow-git safety net before the first task: `@call snapshot("pre-execution")`. (Shadow-git is
a restore point, NOT a worktree substitute.)

**Resume:** on start, `ctx_session load` + `ctx_knowledge recall` — the MCP server auto-injects
the ACTIVE SESSION block. Tasks already marked complete are NOT re-dispatched (recovery map =
`ctx_knowledge` + `git log`).

Once: `ctx_agent action=register agent_type=claude role=plan`.

next: render phase "preflight".
@phase-end

@phase "preflight"

## Preflight — enumerate the plan's tasks (once, before Task 1)

Get the plan's structure without pulling its content into your context and without the
whole-doc render path:

Run: `lean-md render <plan>.lmd.md --list-phases` → an ordered `name<TAB>title` index. This is
import-independent (Bug-3-immune) — it never renders bodies. Create one todo per phase.

**Do NOT `ctx_read` the plan** — any read mode renders it (the source looks empty). The task
brief is always the per-phase render (`render --phase task-N`), never a whole-doc read.

**Pre-flight conflict scan:** skim the plan for internal contradictions or steps that conflict
with the current tree; bundle ALL such concerns into ONE question to the human before Task 1 —
never trickle them out mid-execution.

next: render phase "dispatch-mode".
@phase-end

@phase "dispatch-mode"

## Dispatch Mode — sequential or parallel?

From the preflight enumeration, detect independent task groups: disjoint file/subsystem
sets, no sequential dependency between them.

- **< 2 independent groups** → no question; `next: dispatch` (the sequential path,
  unchanged).
- **≥ 2 independent groups** → ask the human EXACTLY once (bundle it, never trickle):
  > "I found N independent task groups (disjoint files, no shared state): [list].
  > Dispatch them in parallel (one focused subagent per group, each reviewed on
  > return), or run sequentially one task at a time?"
  - Parallel → `next: parallel-dispatch`.
  - Sequential (or any doubt / any coupling) → `next: dispatch`.

next: render phase "dispatch" or "parallel-dispatch" per the answer.
@phase-end

@phase "dispatch"

## Dispatch — one implementer subagent per task

For each task, in order:

1. Record BASE: `BASE = @query "git rev-parse HEAD"` (local git → `@query`, NOT `ctx_git_read`
   which reads remote repos). Note it — Fidelity-critical; never use `HEAD~1` (drops multi-commit
   tasks).
2. Brief = the phase render, captured raw:
   `ctx_shell(command="cargo run -q --bin lean-md -- render <plan> --phase task-N --consumer=ai", raw=true)`.
   Warm the source files the task touches in ONE call: `ctx_multi_read paths=[…]` (the cache is
   shared across session agents — the subagent's first `ctx_read` hits it).
3. Take a snapshot around the edit: `@call snapshot("pre-task-N")` before, `@call
   snapshot("post-task-N")` after — captures exactly what the implementer changed.
4. **Model selection (Controller sets the model at the Agent-tool call; `@dispatch` only composes
   the brief):** mechanical/localized change → cheap model; integration across files → standard;
   architecture/ambiguous → most-capable. `@dispatch` carries NO model param.
5. `@dispatch skill="lmd-subagent-driven-development" companion="implementer" role=dev
   to_agent="{{ controller_id }}"` — the dispatch contract is auto-prepended.
6. **Status handling (faithful to the implementer's report):**
   - DONE → proceed to review.
   - DONE_WITH_CONCERNS → read concerns; address correctness/scope BEFORE review.
   - NEEDS_CONTEXT → supply context, re-dispatch (same or clearer brief).
   - BLOCKED → assess: context-shortfall → more context/same model; reasoning-shortfall → more
     capable model; too-large → split; plan-wrong → escalate to the human. Never re-force the
     same model unchanged.

Continuous execution: do not check in between tasks; stop only on unresolvable BLOCKED, genuine
ambiguity, or "all tasks done". Keep narration to ≤1 line between tool calls.

next: render phase "review".
@phase-end

@phase "review"

## Review — two verdicts from one diff read

1. `HEAD = @query "git rev-parse HEAD"`.
2. `@dispatch skill="lmd-subagent-driven-development" companion="task-reviewer" role=review
   to_agent="{{ controller_id }}"` — pass BASE..HEAD and the plan's Global Constraints verbatim.
   The reviewer reads the brief + the implementer's `ctx_agent` post and fetches the diff itself
   (`@read mode=diff`) — do NOT trust the report; verify against the diff.
3. The reviewer returns two verdicts (Spec-Compliance + Code-Quality) and ✅/❌/⚠️ + Approved/
   Needs-fixes.
4. **⚠️ items** do not block the review — YOU resolve each (you hold cross-task context); a
   confirmed gap = a failed spec review → back to the implementer.
5. **Fix loop** on Critical/Important: dispatch a fix subagent (carries the implementer contract;
   re-runs tests). Plan-mandated findings / plan conflicts → present finding + plan text to the
   human; never silently drop, never dispatch a plan-contradicting fix without asking.
6. Task complete: hand the implementer's `category/key: value` return through the return sink —
   `@call task_return("status: DONE; commits: …")` → distils into parent knowledge — and
   `ctx_session action=task "Task N [x%]"`. (A2A `ctx_task` is optional; if used, the in-progress
   state is `working`, never `in_progress`.)

next: render phase "final-review".
@phase-end

@phase "parallel-dispatch"

## Parallel Dispatch — fan out independent tasks, review each on return

@include parallel-dispatch

Fidelity: the two-verdict review is preserved PER task — only execution fans out.

1. **Per agent, BEFORE fan-out:** record `BASE_i = @query "git rev-parse HEAD"` for each task
   (Fidelity-critical; never `HEAD~1`). Warm the sources each task touches in ONE call:
   `ctx_multi_read paths=[…]` (shared cache).
2. **Fan-out:** emit one `@dispatch skill="lmd-subagent-driven-development"
   companion="implementer" role=dev to_agent="{{ controller_id }}"` per independent task, ALL in
   a single response (multiple in one answer = parallel). Each carries its own `--phase task-N`
   brief; the dispatch contract is auto-prepended. Scopes stay disjoint — no two agents own the
   same file.
3. **Per returning agent:** BASE_i..HEAD review (Spec-Compliance + Code-Quality) — the reviewer
   fetches the diff itself (`@read mode=diff`), never trusts the report. Record progress
   (`ctx_session action=task "Task N [x%]"`) and distil the return
   (`@call task_return("status: …; commits: …")`) per agent, not batched.
4. **Conflict scan:** did more than one agent edit the same file?
   `@query "git diff --name-only BASE..HEAD"` — any overlap → resolve jointly before integrating.
5. Integrate → full suite: `@call gate(<paths>)` — Expected: PASS.
6. Return to the normal flow.

next: render phase "final-review".
@phase-end

@phase "final-review"

## Final Review — whole-branch gate

After the last task, run the mandatory discovery pre-pass, then the LLM judgment:

1. Deterministic pre-pass (Impact/Caller/Test/Smell discovery map):
   `@query "git diff merge-base..HEAD" | @review diff-review`
   plus `@smells` (scan the changed surface).
2. `@dispatch skill="lmd-subagent-driven-development" companion="code-reviewer" role=review
   to_agent="{{ controller_id }}"` (most-capable model) — feed the pre-pass findings as input.
   The companion may optionally pull `ctx_quality action=delta` vs. BASE (objective health
   evidence, not a gate).
3. ONE fix subagent for ALL findings (the complete list, not per-finding).

next: render phase "handoff".
@phase-end

@phase "handoff"

## Handoff — checkpoint & finish

- Phase-boundary context checkpoint: `@call compress()` (controller conversation).
- Branch finishing: invoke the lmd-finishing-a-development-branch skill (merge / PR / keep /
  cleanup choice presented to the human).

This is the terminal phase — there is no "next" render.
@phase-end
