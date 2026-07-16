---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development (lmd delegation stub)

Write the test first. Watch it fail. Write minimal code to pass.
**Core principle:** If you didn't watch the test fail, you don't know if it tests the right thing.
**Violating the letter of the rules is violating the spirit of the rules.**

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

## When to Use
**Always:** new features · bug fixes · refactoring · behavior changes.
**Exceptions (ask your human partner):** throwaway prototypes · generated code · config files.
Thinking "skip TDD just this once"? Stop. That's rationalization.

## The Iron Law
    NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
Wrote code before the test? Delete it. Start over. Delete means delete.

## Phases
- **red** — write one failing test, then Verify RED (watch it fail correctly).
- **green** — minimal code to pass, then Verify GREEN.
- **refactor** — clean up only under green.
- **rationalizations** — read when tempted to skip a step.

## Companions
When adding mocks or test utilities, render the companion to avoid common pitfalls:
- `testing-anti-patterns` — testing mock behavior instead of real behavior, test-only
  methods on production classes, mocking without understanding dependencies.

## Final Rule
    Production code → test exists and failed first
    Otherwise → not TDD
No exceptions without your human partner's permission.
