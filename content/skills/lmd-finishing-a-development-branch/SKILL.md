---
name: lmd-finishing-a-development-branch
description: Native lmd port of the finishing-a-development-branch skill. Render-on-invoke via ctx_md_render(skill="lmd-finishing-a-development-branch", phase=<phase>) against the binary-embedded body — phase-isolated. Guides branch completion by verifying tests, detecting the workspace environment, presenting exactly 4 (or 3 for detached HEAD) integration options, then executing the choice with provenance-based worktree cleanup. Use when implementation is complete and you must decide how to integrate the work — merge, PR, keep, or discard.
---

# lmd-finishing-a-development-branch (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx engine. Do not
read a body or companion file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-finishing-a-development-branch", phase="pre-context")

Interactive, inline (main agent) — no subagent dispatch, no companions. Verifies tests, then
branches the render by the human's chosen integration option.

## Phases (render each as you reach it)

pre-context → verify-tests → detect-env → present-options → then ONE option phase:
merge-local | create-pr | keep-as-is | discard.

Each option phase is terminal (records the close via `ctx_session`); there is no follow-on skill.
