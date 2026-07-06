## Parallel Agent Dispatch

**Core principle:** one dispatch per independent problem domain. Independent = disjoint
files, no shared state, no sequential dependency. Coupled or same-file work stays
sequential — parallelism there corrupts, it does not accelerate.

### When to use (decision gate)
- Multiple problems to solve? → no → single agent, done.
- Independent (disjoint files, no shared state, no ordering)? → no → sequential
  (one dispatch per response), or single agent if truly coupled.
- Independent → **parallel**: fan out one focused agent per domain.

### Fan-out rule
**Multiple dispatches in ONE response = parallel.** One dispatch per response =
sequential. To run agents concurrently, emit every dispatch in a single answer.

### Focused agent task (prompt structure)
Each agent gets exactly:
- **scope** — the files/domain it owns (and the files it must NOT touch),
- **goal** — the single outcome it must produce,
- **constraints** — invariants to hold (tests green, no API break, project rules),
- **output spec** — what to report back (files changed, decisions, blockers).

The Dispatch Contract is prepended to every agent prompt (tool discipline +
register/handoff baton) — never dispatch a bare task.

### Common mistakes (avoid)
- Scope too broad — the agent wanders outside its domain.
- No context — the agent lacks the files/goal to act.
- No constraints — the agent breaks an invariant it never saw.
- Vague output spec — you cannot integrate what you cannot read back.

### Verification (after fan-out)
1. Read each agent's summary/return.
2. Conflict scan — did two agents edit the same file? Resolve before integrating.
3. Run the FULL test suite (never per-agent subsets).
4. Spot-check the integrated result against each domain's goal.

### Memory / coordination (binding — both consumers inherit this)
- Progress: ctx_session action=task "…" per returning agent (not batched at the end).
- Durable facts/gotchas: ctx_knowledge action=remember per agent.
- Baton/status: each subagent ctx_agent action=post category=status +
  ctx_agent action=handoff to_agent=<controller>; the controller uses
  ctx_agent action=sync over the fan-out group, never manual polling.
- Warm-read before dispatch: ctx_multi_read paths=[…] (shared MCP cache; no
  ctx_share, no fresh).

### When NOT to use
Shared state · agents would edit the same files · strict ordering between tasks · a
single agent can hold the whole change. Then: sequential or single-agent.
