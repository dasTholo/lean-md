---
name: lmd-subagent-driven-development
description: Executes a .lmd.md plan one task at a time — a fresh implementer subagent per task, two-verdict review between tasks, a whole-branch final review — all handoffs over lean-ctx memory/coordination. Use when executing an implementation plan with independent tasks in the current session.
---

# lmd-subagent-driven-development (delegation stub)

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

The skill executes a `.lmd.md` plan: one fresh implementer subagent per task, a two-verdict
review (spec-compliance + code-quality) between tasks, and a whole-branch final review. Every
brief, report, diff and baton moves through lean-ctx (`ctx_session`/`ctx_knowledge`/`ctx_agent`)
— never through external SDD bash scripts or scratch-ledger files.

## Phases

orient → preflight → dispatch-mode → dispatch | parallel-dispatch → review → final-review → handoff.

The `dispatch`/`review`/`final-review` phases compose `@dispatch` briefs to the three companions.

## Companions

- `implementer` — per-task implementation brief (TDD, self-review, escalation)
- `task-reviewer` — two-verdict per-task review (spec-compliance + code-quality)
- `code-reviewer` — whole-branch final review
