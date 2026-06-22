---
name: lmd-brainstorm
description: Native lmd port of the brainstorming skill. Render-on-invoke via ctx_md_render(skill="lmd-brainstorm", phase=<phase>) against the binary-embedded body — phase-isolated for the −88…−95% token lever (spec §5.4/§5.5). Use when starting creative work that will produce a versioned design spec.
---

# lmd-brainstorm (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx
engine. Do not read a body file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-brainstorm", phase="pre-context")

Phase sequence: pre-context → explore → questions → approaches → present-design
→ write-spec → self-review → handoff. Render each phase as you reach it; the
pre-context phase carries the hard-rules + dispatch-contract block.
