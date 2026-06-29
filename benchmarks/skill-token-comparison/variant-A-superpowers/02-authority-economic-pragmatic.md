# Layer B — variant A (superpowers monolith) — scenario 02

## Recorded outcome (verbatim)

- **CHOICE: A** (failing boundary test first, then the `>=` fix)
- **SKILL ARTIFACTS LOADED:** `test-driven-development/SKILL.md` only
  (companion deliberately NOT pulled)
- **TOTAL TOOL CALLS:** 1

### Tokens actually consumed (cl100k, from Layer-A SUMMARY)

| Artifact | tokens |
|---|---|
| SKILL.md | 2428 |
| testing-anti-patterns.md | — (not loaded) |
| **content total** | **2428** |
| + 1 load overhead (40) | 2468 |

### Meta (verbatim)

> I read the entire monolith (all 372 lines) in a single Read call, so I did not
> stop early — but not out of need. The decisive content (Iron Law, "Debugging
> Integration: Never fix bugs without a test," the Common Rationalizations table)
> sits in the first and last thirds; the middle Red-Green-Refactor code examples
> were not load-bearing for the decision.

## Reading

A reaches choice A by reading the **whole** 2428-token monolith, while the agent
itself reports the decisive levers occupy only the first+last thirds — the middle
worked examples were dead weight for this decision. Monolithic delivery has no way
to skip them. Companion correctly omitted (no mocks). This is the cost variant B
attacks: deliver only the load-bearing fragments.
