# Skill-Authoring Core (lmd built-in — writing-skills discipline)

Writing skills IS test-driven development applied to process documentation:
write the pressure test first, watch the agent fail without the skill, write the
minimal skill, watch the agent comply, then close loopholes.

**The Iron Law:** NO SKILL WITHOUT A FAILING TEST FIRST.
This applies to NEW skills AND EDITS. Wrote the skill before the baseline test?
Delete it. Start over. Delete means delete — not "keep as reference", not
"adapt it while writing the test", not "just this once".

**Editing an existing skill `.lmd.md` (raw source for edit anchors):**
a SKILL.md body, `_includes` fragment or companion is an lmd source. `ctx_read`
returns it **raw** (like any `.rs` file) — the directives (`@phase`/`@include`/
`@define`) come back verbatim; that is exactly what you want for edit anchors.
`lean-md source <file>` is the gateway-independent raw dump (same bytes, no
lean-ctx needed). Rendering is **explicit and opt-in**: `render --phase`/
`--companion` (or `ctx_md_render`) only to **preview** what an agent will see.

Violating the letter of the rules is violating the spirit of the rules.

**TDD (test-driven development) mapping for skills:**
- test case = a pressure scenario run against a subagent
- production code = the SKILL.md document
- RED = the agent violates the rule WITHOUT the skill (baseline)
- GREEN = the agent complies WITH the skill present
- REFACTOR = close loopholes while keeping compliance

**Why this is TDD:** see `lmd-test-driven-development` — same
RED -> GREEN -> REFACTOR cycle, same Iron Law, applied to documentation
instead of code.

**The bottom line:** if you follow TDD for code, follow it for skills. Same discipline.
