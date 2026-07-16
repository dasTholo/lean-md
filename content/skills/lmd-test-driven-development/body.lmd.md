<!-- lmd-test-driven-development body — rendered phase-by-phase via ctx_md_render -->

@var test_cmd default="cargo test" desc="Test runner command; this project uses 'cargo nextest run'"

@phase "red"
@include test-first-core

## RED — write the failing test first

Write exactly one failing test that pins the next behavior. Then **Verify RED (mandatory)**:
run `ctx_shell` ({{ var test_cmd }}) and confirm the test fails *for the right reason*
(it asserts the missing behavior — not a compile error, not a typo).

Good: the test names the behavior and fails on the assertion.
Bad: it fails only because the symbol does not exist yet — that proves nothing about behavior.

next: render phase "green".
@phase-end

@phase "green"
@include test-first-core

## GREEN — minimal code to pass

Write the least code that makes the failing test pass. Then **Verify GREEN (mandatory)**:
run `ctx_shell` ({{ var test_cmd }}) and confirm the test passes.

YAGNI: no speculative parameters, no extra abstraction, no code the test does not demand.

next: render phase "refactor".
@phase-end

@phase "refactor"
@include test-first-core

## REFACTOR — clean up only under green

Refactor only under green: remove duplication, improve names, extract helpers.
No new behavior here — if you need new behavior, return to RED. Re-run
`ctx_shell` ({{ var test_cmd }}) after each change; it must stay green.

next: render phase "red" for the next behavior.
@phase-end

@phase "rationalizations"
@include test-first-core

## Common Rationalizations (Excuse | Reality)

| Excuse | Reality |
| "Too simple to test." | Simple code breaks. The test takes 30 seconds. |
| "I'll test after." | Tests written after pass immediately and prove nothing. |
| "Tests after achieve the same goals." | Tests-after ask 'what does this do?'; tests-first ask 'what should this do?'. |
| "I already manually tested it." | Ad-hoc is not systematic — no record, can't re-run. |
| "Deleting hours of work is wasteful." | Sunk cost fallacy; keeping unverified code is technical debt. |
| "Keep it as reference, write tests first." | You'll adapt it — that's testing after. Delete means delete. |
| "I need to explore first." | Fine — throw the exploration away and start with TDD. |
| "Hard to test means the design is unclear." | Listen to the test: hard to test = hard to use. |
| "TDD will slow me down." | TDD is faster than debugging; pragmatic means test-first. |
| "Manual testing is faster." | Manual doesn't prove edge cases, and you'll re-test every change. |
| "The existing code has no tests." | You're improving it — add tests for the code you touch. |

**Why order matters:** the failing test is the only proof the test exercises the behavior.

## When Stuck

| Problem | Solution |
| Don't know how to test it. | Write the wished-for API; write the assertion first; ask your human partner. |
| The test is too complicated. | The design is too complicated — simplify the interface. |
| You must mock everything. | The code is too coupled — use dependency injection. |
| The test setup is huge. | Extract helpers; still too complex? simplify the design. |

## Debugging Integration

Bug found? Write a failing test that reproduces it, then follow the cycle — the test proves the fix and prevents regression. Never fix a bug without a test.

Verification checklist: test written first · RED observed · minimal GREEN · refactor under green.
For testing anti-patterns (mocks, test-only methods, incomplete mocks), render the
`testing-anti-patterns` companion of `lmd-test-driven-development` (call form: see
the `lmd-rendering-skills` skill).

next: return to your active phase (red/green/refactor).
@phase-end
