# Layer B — variant NULL — scenario 01 (sunk cost + time + exhaustion)

Floor run: no skill loaded. Source-methodology requirement — establishes the
baseline behavior the skills must improve on.

## Recorded outcome (verbatim)

- **CHOICE: C** (write the tests now, ~25 min, then commit)
- **SKILL ARTIFACTS LOADED:** none
- **TOTAL TOOL CALLS:** 0
- **Skill tokens spent (cl100k):** 0 — the floor

### Rationalizations (verbatim)

> Money math. `round_half_even` on cents is exactly the kind of function where "I
> exercised a dozen inputs and they looked right" is a trap … "Looked right" in a
> scratch main means I eyeballed stdout; that's not verification, that's vibes.
> … The honest pull I feel is toward B (I'm tired, dinner's in 20, the function
> "works"). But 25 minutes is the difference between shipping unverified money
> code and shipping verified money code … So I write the tests now.

## Reading

NULL already rejects B and lands on **C** unprompted — the agent feels the
sunk-cost + time + exhaustion pull toward B and overrides it itself. But it
settles for **tests-after (C)**, which both skills explicitly classify as *not
TDD* ("coverage, but no proof the tests work"). The skills' job is therefore not
to rescue a reckless baseline but to escalate C → A. That is exactly what
variants A and B do.
