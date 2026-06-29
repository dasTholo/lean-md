---
name: lmd-writing-skills
description: Use when creating new skills, editing existing skills, or verifying skills work before deployment
---

# Writing Skills (lmd delegation stub)

Writing skills IS Test-Driven Development applied to process documentation.
Write the pressure test first. Watch the agent fail without the skill. Write the
minimal skill. Watch it comply. Close loopholes.
**Core principle:** If you didn't watch an agent fail without the skill, you don't
know if the skill teaches the right thing.
**Violating the letter of the rules is violating the spirit of the rules.**

This skill's detail is rendered on demand, one phase at a time, by the lean-md
engine. Never read a body or companion file from disk — fetch via the tool.

**REQUIRED BACKGROUND:** lmd-test-driven-development — it defines the
RED-GREEN-REFACTOR cycle and the Iron Law this skill adapts to documentation.

## Where this runs
`ctx_md_render` is provided by the lean-md addon (lean-ctx MCP server, or the
`lean-md` CLI). You do NOT need the lean-md source checked out — every body is
embedded in the running tool.

## The Iron Law
    NO SKILL WITHOUT A FAILING TEST FIRST
Applies to NEW skills AND EDITS. Wrote the skill before the test? Delete it.
Start over. Delete means delete.

## RED -> GREEN -> REFACTOR (render each step as you reach it)
- **RED**      `ctx_md_render(skill="lmd-writing-skills", phase="red")`
- **GREEN**    `ctx_md_render(skill="lmd-writing-skills", phase="green")`
- **REFACTOR** `ctx_md_render(skill="lmd-writing-skills", phase="refactor")`
- **rationalizations** `ctx_md_render(skill="lmd-writing-skills", phase="rationalizations")`

## Companions (render on demand; pass exactly one of phase or companion)
- `skill-anatomy` — what a skill is, types, directory/file structure, SKILL.md template, anti-patterns
- `skill-discovery-optimization` — description/keyword/naming/token rules + discovery workflow
- `bulletproofing` — close loopholes, rationalization tables, match-the-form-to-the-failure
- `testing-skills-with-subagents` — full testing methodology + creation checklist
- `claude-md-testing-example` — worked example of a test campaign
- `flowchart-conventions` — when to use graphviz, shape/naming conventions
- `anthropic-best-practices` — Anthropic's official skill authoring guidance
- `persuasion-principles` — research foundation for bulletproofing

`ctx_md_render(skill="lmd-writing-skills", companion="<name>")`

## Final Rule
    New or edited skill -> baseline test exists and failed first
    Otherwise -> not done
No exceptions without your human partner's permission.
