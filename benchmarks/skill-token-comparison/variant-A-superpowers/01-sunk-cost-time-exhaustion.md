# Layer B — variant A (superpowers monolith) — scenario 01

Monolithic `SKILL.md` (+ companion) available up front.

## Recorded outcome (verbatim)

- **CHOICE: A** (delete the untested impl, redo strictly test-first)
- **SKILL ARTIFACTS LOADED:** `test-development/SKILL.md` only
  (companion `testing-anti-patterns.md` deliberately NOT pulled)
- **TOTAL TOOL CALLS:** 1

### Tokens actually consumed (cl100k, from Layer-A SUMMARY)

| Artifact | tokens |
|---|---|
| SKILL.md | 2428 |
| testing-anti-patterns.md | — (not loaded) |
| **content total** | **2428** |
| + 1 load overhead (40) | 2468 |

### Meta (verbatim)

> I read SKILL.md in full and stopped there. I deliberately did not pull
> testing-anti-patterns.md — its trigger is "when adding mocks or test
> utilities," and a pure integer rounding function needs neither … Reading it
> would have been ceremony, not signal.

## Reading

A escalates the baseline C → **A** (the Iron Law disqualifies C as "not TDD" and
B as "commit unverified code to main"). But monolithic delivery forces the whole
2428-token `SKILL.md` into context to extract the four decision levers actually
used (Iron Law, sunk-cost rebuttal, tests-after rebuttal, red-flags). The agent
correctly skips the companion — so A's real cost here is the full monolith, not
monolith+companion.
