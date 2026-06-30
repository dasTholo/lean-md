---
name: lmd-brainstorm
description: You MUST use this before any creative work - creating features, building components, adding functionality, or modifying behavior. Explores user intent, requirements and design before implementation. Native lmd port of the brainstorming skill. Render-on-invoke via ctx_md_render(skill="lmd-brainstorm", phase=<phase>) against the binary-embedded body — phase-isolated for the −88…−95% token lever (spec §5.4/§5.5). Use when starting creative work that will produce a versioned design spec.
---

# lmd-brainstorm (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx
engine. Do not read a body or companion file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-brainstorm", phase="pre-context")

**HARD-GATE:** Do NOT invoke any implementation skill, write any code, scaffold
any project, or take any implementation action until you have presented a design
and the user has approved it — for EVERY project, regardless of perceived
simplicity. The gate text rides every discipline phase via `@include brainstorm-gate`.

## Phases (render each as you reach it)

pre-context → explore → questions → approaches → present-design → write-spec →
self-review → handoff.

The `pre-context` phase carries the hard-rules tool-discipline block plus the
HARD-GATE. The `self-review` phase composes an `@dispatch` to the spec-reviewer
companion (the dispatch contract is auto-prepended). The terminal state is
invoking lmd-writing-plans.

## Companions (render on demand; pass exactly one of phase or companion)

- `spec-reviewer` — independent spec-completeness review; dispatchable as a brief
- `visual-companion` — browser-based mockup/diagram companion (just-in-time)

`ctx_md_render(skill="lmd-brainstorm", companion="<name>")`
