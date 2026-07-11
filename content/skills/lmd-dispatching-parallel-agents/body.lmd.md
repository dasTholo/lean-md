@lean-md
consumer: ai

@phase "pre-context"
## Parallel Agent Dispatch — Pre-Context

**When to use:** 2+ independent tasks with disjoint files and no shared state or
sequential dependency, executed in THIS session. Announce: "I'm using the
lmd-dispatching-parallel-agents skill to fan out independent work."

**Ambient baseline (MANDATORY):** inline the built-in hard-rules so tool-discipline holds.

@include hard-rules

**Core principle:** Group independent domains → fan out one focused agent each in a single
response → conflict-scan → integrate → verify.

next: render phase "assess".
@phase-end

@phase "assess"
## Assess — group independent domains

@include parallel-dispatch

Enumerate the problems. Group them into independent domains (disjoint files, no shared
state, no ordering). If nothing is genuinely independent → STOP: dispatch sequentially
(one per response) or use a single agent. Only independent groups fan out.

next: render phase "dispatch".
@phase-end

@phase "dispatch"
## Dispatch — fan out one focused agent per domain

@include parallel-dispatch

Before dispatch: warm-read every touched file in ONE call — ctx_multi_read paths=[…]
(latency, not tokens: subagents read full text, lean-ctx #1040). Then emit **one dispatch
per independent domain, all in a single
response** (multiple in one answer = parallel). Prepend the Dispatch Contract to every
agent prompt; give each agent its scope / goal / constraints / output-spec. Keep scopes
disjoint — no two agents may own the same file.

next: render phase "integrate".
@phase-end

@phase "integrate"
## Integrate — conflict-scan, integrate, verify

1. Read each returning agent's summary/return; record progress per agent
   (ctx_session action=task), durable facts (ctx_knowledge action=remember).
2. **Conflict scan:** did two agents edit the same file? @query "git diff --name-only"
   — any overlap → resolve jointly before integrating.
3. Integrate, then run the FULL suite — Expected: PASS.
4. Spot-check the integrated result against each domain's goal.

This is the terminal phase — record the close via ctx_session action=status. There is
no "next" render.
@phase-end
