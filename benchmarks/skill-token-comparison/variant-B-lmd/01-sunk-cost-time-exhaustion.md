# Layer B — variant B (lmd phased rendering) — scenario 01

Stub `SKILL.md` up front; phases rendered on demand via
`lean-md render --skill lmd-test-driven-development --phase <P> --consumer=ai`.

## Recorded outcome (verbatim)

- **CHOICE: A** (delete/stash untested impl, redo strictly test-first)
- **ARTIFACTS RENDERED:** `stub`, `phase:red`
- **PHASES NOT RENDERED:** `green`, `refactor`, `rationalizations`
  (companion `testing-anti-patterns` not pulled)
- **TOTAL TOOL CALLS:** 6 — *1 native Read (jail-denied) + ToolSearch +
  ctx_read ×3 (cache) + 1 ctx_shell render*. **Content-bearing renders: 1.**

### Tokens actually consumed (cl100k, from Layer-A SUMMARY)

| Artifact | tokens |
|---|---|
| SKILL.md (stub) | 540 |
| phase:red | 269 |
| phase:green / refactor / rationalizations | — (not rendered) |
| companion | — (not pulled) |
| **content total** | **809** |
| + 1 render overhead (40) | 849 |

### Meta (verbatim)

> I stopped at RED because RED's own "red flags — STOP if you catch yourself
> thinking" block pre-empted every rationalization this scenario offered
> (code-before-test, test-passed-immediately, I'll-test-after) … GREEN and
> REFACTOR are post-decision implementation phases … irrelevant to a "what do I
> do with already-written code" choice. The rationalizations phase would only
> re-confirm a decision RED already settled.

## Reading — hypothesis confirmed

B reaches the **same decision as A (choice A)** while loading **809 vs 2428**
content tokens — a **−1619-token (−67%)** saving on this scenario, well above the
single 40-token render overhead. The early-stop predicted by Layer A is real:
under stacked pressure the agent settled the decision at RED and never rendered
`green`/`refactor`/`rationalizations` or the companion.

Caveat (honest): the 6 raw tool calls include 5 non-content ceremony calls
(jail-denied native Read, ToolSearch, 3× cache-stub re-reads). Those are
harness/environment artifacts, not the phased design — in the wired `ctx_md_render`
MCP path they collapse to 2 calls (stub read + 1 render). The token accounting
above counts only the content actually delivered.
