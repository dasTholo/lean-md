<!-- lmd-test-driven-development body — rendered phase-by-phase via ctx_md_render -->

@phase "red"
@include test-first-core

## RED — write the failing test first

Write exactly one failing test that pins the next behavior. Then **Verify RED (mandatory)**:
run `ctx_shell "cargo nextest run"` and confirm the test fails *for the right reason*
(it asserts the missing behavior — not a compile error, not a typo).

Good: the test names the behavior and fails on the assertion.
Bad: it fails only because the symbol does not exist yet — that proves nothing about behavior.
@phase-end

@phase "green"
@include test-first-core

## GREEN — minimal code to pass

Write the least code that makes the failing test pass. Then **Verify GREEN (mandatory)**:
run `ctx_shell "cargo nextest run"` and confirm the test passes.

YAGNI: no speculative parameters, no extra abstraction, no code the test does not demand.
@phase-end

@phase "refactor"
@include test-first-core

## REFACTOR — clean up only under green

Refactor only under green: remove duplication, improve names, extract helpers.
No new behavior here — if you need new behavior, return to RED. Re-run
`ctx_shell "cargo nextest run"` after each change; it must stay green.
@phase-end

@phase "rationalizations"
@include test-first-core

## Common Rationalizations (Excuse | Reality)

| Excuse | Reality |
| "I'll test after." | Code-after-test is production code without a failing test. |
| "It's too simple to break." | Simple code breaks; the test is cheap. |
| "The test passed right away." | It never failed — no evidence it tests anything. |
| "Refactor needs a quick prod tweak." | A prod tweak is new behavior — go back to RED. |

**Why order matters:** the failing test is the only proof the test exercises the behavior.
**When stuck:** shrink the test until one tiny behavior is in scope, then RED → GREEN.

Verification checklist: test written first · RED observed · minimal GREEN · refactor under green.
(For testing anti-patterns, see the companion ported in Spec #2 — `testing-anti-patterns`.)
@phase-end
