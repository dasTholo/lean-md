---
name: lmd-dispatching-parallel-agents
description: Native lmd port of the dispatching-parallel-agents skill. Render-on-invoke via ctx_md_render(skill="lmd-dispatching-parallel-agents", phase=<phase>) against the binary-embedded body — phase-isolated. Fans out one focused subagent per independent problem domain (disjoint files, no shared state), then conflict-scans and integrates. Use when facing 2+ independent tasks that can be worked on without shared state or sequential dependencies.
---

# lmd-dispatching-parallel-agents (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx engine. Do not
read a body or companion file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-dispatching-parallel-agents", phase="pre-context")

Interactive, inline (main agent) — no follow-on skill, no companions. Groups independent
domains, fans out one focused agent per domain in a single response, then integrates.

## Phases (render each as you reach it)

pre-context → assess → dispatch → integrate.

The single-source fan-out guidance (when-to-use gate, fan-out rule, prompt structure,
memory/coordination block) is the built-in `parallel-dispatch` fragment, shared with
lmd-subagent-driven-development via `@include parallel-dispatch`.
