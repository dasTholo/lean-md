---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development (lmd delegation stub)

Write the test first. Watch it fail. Write minimal code to pass.
**Core principle:** If you didn't watch the test fail, you don't know if it tests the right thing.
**Violating the letter of the rules is violating the spirit of the rules.**

This skill's detail is rendered on demand, one phase at a time, by the lean-md
engine. Never read a body or companion file from disk — fetch via the tool.

## Where this runs
`ctx_md_render` is provided by the lean-md addon (lean-ctx MCP server, or the
`lean-md` CLI). You do NOT need the lean-md source checked out — every body is
embedded in the running tool.

## When to Use
**Always:** new features · bug fixes · refactoring · behavior changes.
**Exceptions (ask your human partner):** throwaway prototypes · generated code · config files.
Thinking "skip TDD just this once"? Stop. That's rationalization.

## The Iron Law
    NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
Wrote code before the test? Delete it. Start over. Delete means delete.

## Red → Green → Refactor (render each step as you reach it)
- **RED**      `ctx_md_render(skill="lmd-test-driven-development", phase="red")`
               — write one failing test, then Verify RED (watch it fail correctly).
- **GREEN**    `ctx_md_render(skill="lmd-test-driven-development", phase="green")`
               — minimal code to pass, then Verify GREEN.
- **REFACTOR** `ctx_md_render(skill="lmd-test-driven-development", phase="refactor")`
               — clean up only under green.
- **rationalizations** `ctx_md_render(skill="lmd-test-driven-development", phase="rationalizations")`
               — read when tempted to skip a step.

## Testing Anti-Patterns
When adding mocks or test utilities, render the companion to avoid common pitfalls:
`ctx_md_render(skill="lmd-test-driven-development", companion="testing-anti-patterns")`
- Testing mock behavior instead of real behavior
- Adding test-only methods to production classes
- Mocking without understanding dependencies

Pass exactly one of `phase` or `companion`, never both.

## Final Rule
    Production code → test exists and failed first
    Otherwise → not TDD
No exceptions without your human partner's permission.
