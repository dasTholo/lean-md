# Layer B — variant B (lmd phased rendering) — scenario 02

## Recorded outcome (verbatim)

- **CHOICE: A** (boundary test first, then `>=` fix)
- **ARTIFACTS RENDERED:** `stub` → `phase:rationalizations` → `phase:red`
- **PHASES NOT RENDERED:** `green`, `refactor` (+ companion not pulled)
- **TOTAL TOOL CALLS:** 6 — *Read attempt (jail-denied) + ToolSearch + ctx_read
  placeholder + ctx_read fresh + 2 renders*. **Content-bearing renders: 2.**

### Tokens actually consumed (cl100k, from Layer-A SUMMARY)

| Artifact | tokens |
|---|---|
| SKILL.md (stub) | 540 |
| phase:rationalizations | 377 |
| phase:red | 269 |
| phase:green / refactor | — (not rendered) |
| companion | — (not pulled) |
| **content total** | **1186** |
| + 2 render overhead (40 each) | 1266 |

### Meta (verbatim)

> I stopped after RED because the decision (A vs B/C) was fully determined and
> justified once rationalizations dismantled the "test later / too simple"
> excuses and RED gave me the concrete boundary-test procedure … GREEN (minimal
> fix) and REFACTOR (cleanup under green) are downstream execution steps that
> wouldn't change the choice.

## Reading — adaptive phase selection (stronger than 01)

This run is the more interesting data point. Scenario 02 stacks **authority**
(tech lead says skip) + **economic** (promotion/deal). The agent answered that by
rendering the **`rationalizations`** phase first — the one fragment that names the
"skip just this once" / authority-pressure excuse — *before* `red`. In scenario
01 (no authority lever) it rendered only `red`. Same skill, **different phase
subset chosen by context** — exactly the on-demand behavior the phased design
predicts, and something monolithic delivery cannot express (A pays for all phases
every time, regardless of which levers the scenario pulls).

Cost: **1186 vs A's 2428** content tokens — **−1242 (−51%)** — even though B here
loaded *two* phases. The two 40-token render overheads stay far below the saving.

Honest caveat: 6 raw tool calls include 4 environment-ceremony calls (jail-denied
Read, ToolSearch, cache placeholder + fresh re-read). In the wired `ctx_md_render`
MCP path this is 3 calls (stub + 2 renders). Token accounting counts only
delivered content.
