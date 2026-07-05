---
name: lmd-executing-plans
description: Native lmd inline-execution skill. Render-on-invoke via ctx_md_render(skill="lmd-executing-plans", phase=<phase>) against the binary-embedded body — phase-isolated. Executes a .lmd.md plan in this session — the main agent runs each task itself, pauses at batch checkpoints for human review, and closes at a whole-branch final gate. No companions. Use when executing an implementation plan inline with human review checkpoints.
---

# lmd-executing-plans (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx engine. Do
not read a body file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-executing-plans", phase="orient")

The skill executes a `.lmd.md` plan inline: the main agent runs each task itself, pauses at
executor-chosen batch boundaries for human review, and closes at a whole-branch final gate.
Every brief, diff and progress marker moves through lean-ctx (`ctx_session`/`ctx_knowledge`
/`ctx_agent`) — never through external SDD bash scripts or scratch-ledger files.

## Phases (render each as you reach it)

orient → preflight → execute → checkpoint → final-gate → finish.

The `orient` phase inlines the ambient tool-discipline baseline via `@include hard-rules`
(the subagent-only register/handoff contract does not apply to inline execution).

## Companions

None — this is the inline-execution variant. For per-task subagent dispatch with a
two-verdict reviewer, use lmd-subagent-driven-development instead.
