---
name: lmd-writing-plans
description: Native lmd port of the writing-plans skill. Render-on-invoke via ctx_md_render(skill="lmd-writing-plans", phase=<phase>) against the binary-embedded body — phase-isolated. Produces token-efficient `.lmd.md` implementation plans (task-on-demand render, code anchors, @define/@call macros) without functional loss. Use when you have a spec or requirements for a multi-step task, before touching code.
---

# lmd-writing-plans (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx
engine. Do not read a body or companion file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-writing-plans", phase="pre-context")

The skill produces plans as `.lmd.md` documents: a meta-head (Goal / Architecture /
Global Constraints + `@var` + `@import .lean-ctx/lean-md/plan-recipes /`), one
`@phase "task-N"` per task, anchors for existing code, verbatim for new code, and
`@call` macros for boilerplate. The executing controller renders one task at a
time: `lean-md render <plan.lmd.md> --phase task-N`.

## Phases (render each as you reach it)

pre-context → file-structure → task-sizing → plan-format → write-plan →
self-review → handoff.

The `self-review` phase composes an `@dispatch` to the plan-reviewer companion (the
dispatch contract is auto-prepended). The terminal state offers the execution
handoff (lmd-subagent-driven-development or lmd-executing-plans).

## Companions (render on demand; pass exactly one of phase or companion)

- `plan-reviewer` — independent plan-completeness review; dispatchable as a brief

`ctx_md_render(skill="lmd-writing-plans", companion="plan-reviewer")`
