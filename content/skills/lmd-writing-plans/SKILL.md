---
name: lmd-writing-plans
description: Produces token-efficient `.lmd.md` implementation plans (task-on-demand render, code anchors, @define/@call macros) without functional loss. Use when you have a spec or requirements for a multi-step task, before touching code.
---

# lmd-writing-plans (delegation stub)

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

The skill produces plans as `.lmd.md` documents: a meta-head (Goal / Architecture /
Global Constraints + `@var` + `@import .lean-ctx/lean-md/plan-recipes /`), one
`@phase "task-N"` per task, anchors for existing code, verbatim for new code, and
`@call` macros for boilerplate. The executing controller renders one task at a
time: `lean-md render <plan.lmd.md> --phase task-N`.

## Phases

pre-context → file-structure → task-sizing → plan-format → write-plan →
self-review → handoff.

The `self-review` phase composes an `@dispatch` to the plan-reviewer companion. The
terminal state offers the execution handoff (lmd-subagent-driven-development or
lmd-executing-plans).

## Companions

- `plan-reviewer` — independent plan-completeness review; dispatchable as a brief
