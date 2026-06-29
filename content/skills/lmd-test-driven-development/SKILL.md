---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# lmd-test-driven-development (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx
engine. Do not read a body file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-test-driven-development", phase="red")

Phase sequence: red → green → refactor → rationalizations. Render each phase as
you reach it; every phase carries the test-first-core discipline block.
