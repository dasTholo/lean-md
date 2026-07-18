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

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

**REQUIRED BACKGROUND:** lmd-test-driven-development — it defines the
RED-GREEN-REFACTOR cycle and the Iron Law this skill adapts to documentation.

## The Iron Law
    NO SKILL WITHOUT A FAILING TEST FIRST
Applies to NEW skills AND EDITS. Wrote the skill before the test? Delete it.
Start over. Delete means delete.

## Phases

red → green → refactor. The `rationalizations` phase is read when tempted to skip a step.

## Companions
- `skill-anatomy` — what a skill is, types, directory/file structure, SKILL.md template, anti-patterns
- `skill-discovery-optimization` — description/keyword/naming/token rules + discovery workflow
- `bulletproofing` — close loopholes, rationalization tables, match-the-form-to-the-failure
- `testing/methodology` — RED→GREEN→REFACTOR testing workflow (pressure scenarios, rationalization tables)
- `testing/skill-types` — how to test discipline/technique/pattern/reference skills
- `testing/creation-checklist` — TDD (test-driven development)-adapted checklist before deploying a skill
- `claude-md-testing-example` — worked example of a test campaign
- `flowchart-conventions` — when to use graphviz, shape/naming conventions
- `anthropic-best-practices` — Anthropic's official skill authoring guidance
- `persuasion-principles` — research foundation for bulletproofing

## Final Rule
    New or edited skill -> baseline test exists and failed first
    Otherwise -> not done
No exceptions without your human partner's permission.
