---
name: lmd-dispatching-parallel-agents
description: Fans out one focused subagent per independent problem domain (disjoint files, no shared state), then conflict-scans and integrates. Use when facing 2+ independent tasks that can be worked on without shared state or sequential dependencies.
---

# lmd-dispatching-parallel-agents (delegation stub)

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

Interactive, inline (main agent) — no follow-on skill, no companions. Groups independent
domains, fans out one focused agent per domain in a single response, then integrates.

## Phases

pre-context → assess → dispatch → integrate.

The single-source fan-out guidance (when-to-use gate, fan-out rule, prompt structure,
memory/coordination block) is the built-in `parallel-dispatch` fragment, shared with
lmd-subagent-driven-development via `@include parallel-dispatch`.
