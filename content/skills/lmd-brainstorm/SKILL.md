---
name: lmd-brainstorm
description: You MUST use this before any creative work - creating features, building components, adding functionality, or modifying behavior. Explores user intent, requirements and design before implementation. Use when starting creative work that will produce a versioned design spec.
---

# lmd-brainstorm (delegation stub)

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

**HARD-GATE:** Do NOT invoke any implementation skill, write any code, scaffold
any project, or take any implementation action until you have presented a design
and the user has approved it — for EVERY project, regardless of perceived
simplicity. The gate text rides every discipline phase via `@include brainstorm-gate`.

## Phases

pre-context → explore → questions → approaches → present-design → write-spec →
self-review → handoff.

The `pre-context` phase carries the hard-rules tool-discipline block plus the
HARD-GATE. The `self-review` phase composes an `@dispatch` to the spec-reviewer
companion. The terminal state is invoking lmd-writing-plans.

## Companions

- `spec-reviewer` — independent spec-completeness review; dispatchable as a brief
- `visual-companion` — browser-based mockup/diagram companion (just-in-time)
