---
name: lmd-subagent-driven-development
description: Native lmd port of the subagent-driven-development skill. Render-on-invoke via ctx_md_render(skill="lmd-subagent-driven-development", phase=<phase>) against the binary-embedded body — phase-isolated. Executes a .lmd.md plan one task at a time — a fresh implementer subagent per task, two-verdict review between tasks, a whole-branch final review — all handoffs over lean-ctx memory/coordination. Use when executing an implementation plan with independent tasks in the current session.
---

# lmd-subagent-driven-development (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx engine. Do not
read a body or companion file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-subagent-driven-development", phase="orient")

The skill executes a `.lmd.md` plan: one fresh implementer subagent per task, a two-verdict
review (spec-compliance + code-quality) between tasks, and a whole-branch final review. Every
brief, report, diff and baton moves through lean-ctx (`ctx_session`/`ctx_knowledge`/`ctx_agent`)
— never through external SDD bash scripts or scratch-ledger files.

## Phases (render each as you reach it)

orient → preflight → dispatch-mode → dispatch | parallel-dispatch → review → final-review → handoff.

The `dispatch`/`review`/`final-review` phases compose `@dispatch` briefs to the three companions
(the dispatch contract is auto-prepended).

## Companions (render on demand; pass exactly one of phase or companion)

- `implementer` — per-task implementation brief (TDD, self-review, escalation)
- `task-reviewer` — two-verdict per-task review (spec-compliance + code-quality)
- `code-reviewer` — whole-branch final review

`ctx_md_render(skill="lmd-subagent-driven-development", companion="implementer")`
