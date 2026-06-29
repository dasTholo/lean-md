# Test-First Core (lmd built-in — TDD (test-driven development) discipline)

This is test-driven development: write the test first, watch it fail, then make it pass.
**Core principle:** if you didn't watch the test fail, you don't know it tests the right thing.

**The Iron Law:** NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST.
Delete means delete: if you delete a test, you delete the behavior it covered.

Violating the letter of the rules is violating the spirit of the rules.

**Red flags — STOP and start over if you catch yourself thinking:**
- "Code before test" — the test comes first, always.
- "Test after implementation" — that is not TDD; delete it and restart.
- "The test passed immediately" — then it never failed; you have no proof it tests anything.
- "Can't explain why the test failed" — you do not yet understand what you are testing.
- "I'll add the tests later" — later never proves the code; tests come first.
- "Just this once" — rationalizing the skip is the skip.
- "I already manually tested it" — ad-hoc is not systematic; no record, can't re-run.
- "Tests after achieve the same purpose" — tests-after ask 'what does this do?'; tests-first ask 'what should this do?'.
- "It's about spirit not ritual" — violating the letter is violating the spirit.
- "Keep it as reference / adapt the existing code" — you'll adapt it; that's testing after. Delete means delete.
- "Already spent hours, deleting is wasteful" — sunk cost; unverified code is technical debt.
- "TDD is dogmatic, I'm being pragmatic" — TDD is the pragmatic path; shortcuts mean debugging in production.
- "This is different because…" — it isn't. Write the failing test first.
