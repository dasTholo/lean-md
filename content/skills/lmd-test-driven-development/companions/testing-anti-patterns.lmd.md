# Testing Anti-Patterns (lmd companion — load when writing/changing tests or adding mocks)

@include test-first-core

Test what the code does, not what the mocks do. Mocks isolate; they are not the thing under test.

## The Iron Laws
1. NEVER test mock behavior.
2. NEVER add test-only methods to production code.
3. NEVER mock without understanding the dependency.

## Anti-Pattern 1 — Testing mock behavior
Asserting that a mock exists proves the mock works, not that the code works. Never assert on `*-mock` ids/handles.
Gate: BEFORE asserting on a mock — "real behavior, or just mock existence?" IF existence: STOP — delete the assertion or unmock the component.

## Anti-Pattern 2 — Test-only methods in production
A method only ever called from tests (cleanup/`destroy`-style) pollutes the production type and can fire in production.
Gate: BEFORE adding a method — "only used by tests?" IF yes: STOP — put it in test utilities. "Does this type own this resource's lifecycle?" IF no: STOP — wrong type.

## Anti-Pattern 3 — Mocking without understanding
Mocking away a method whose side effect the test depends on makes the test pass (or fail) for the wrong reason.
Gate: BEFORE mocking — "what side effects does the real method have, and does the test depend on them?" IF yes: mock at the lower (slow/external) level, not the method the test needs. IF unsure: run with the real impl first, then mock minimally. Red flag: "I'll mock this to be safe."

## Anti-Pattern 4 — Incomplete mocks
A partial mock with only the fields you know about fails silently when downstream code reads an omitted field.
Gate: BEFORE building a mock response — mirror the COMPLETE real structure, every field the system may consume. If uncertain, include all documented fields.

## Anti-Pattern 5 — Integration tests as afterthought
"Implementation complete, tests later" is not done. Testing is part of implementation, not an optional follow-up.
Gate: failing test first → minimal code → refactor → THEN claim complete.

## Quick Reference
| Anti-pattern | Fix |
| Assert on mock elements | Test real behavior or unmock it |
| Test-only methods in production | Move to test utilities |
| Mock without understanding | Understand dependencies first, mock minimally |
| Incomplete mocks | Mirror the real structure completely |
| Tests as afterthought | TDD — tests first |
| Over-complex mocks | Prefer integration tests with real components |

## Red Flags
- Assertions checking for `*-mock` ids.
- Methods only called from test files.
- Mock setup is >50% of the test.
- The test fails when you remove the mock.
- Can't explain why the mock is needed; "mocking just to be safe".

Bottom line: mocks are tools to isolate, not things to test. If TDD reveals you are testing a mock, test real behavior or question why you are mocking at all.
