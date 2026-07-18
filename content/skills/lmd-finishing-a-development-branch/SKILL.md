---
name: lmd-finishing-a-development-branch
description: Guides branch completion by verifying tests, detecting the workspace environment, presenting exactly 4 (or 3 for detached HEAD) integration options, then executing the choice with provenance-based worktree cleanup. Use when implementation is complete and you must decide how to integrate the work — merge, PR, keep, or discard.
---

# lmd-finishing-a-development-branch (delegation stub)

Body renders one phase at a time — never read it from disk.
Rendering, diagnosis and fallback: see the `lmd-rendering-skills` skill.

Interactive, inline (main agent) — no subagent dispatch, no companions. Verifies tests, then
branches the render by the human's chosen integration option.

## Phases

pre-context → verify-tests → detect-env → present-options → then ONE option phase:
merge-local | create-pr | keep-as-is | discard.

Each option phase is terminal (records the close via `ctx_session`); there is no follow-on skill.
