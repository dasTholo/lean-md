# Subagent-Driven Multi-Agent Execution — lean-ctx Contract

CRITICAL: This applies whenever a plan is executed via
`superpowers:subagent-driven-development` (controller dispatches one fresh
subagent per task with a self-crafted, isolated prompt).

`AGENTS.md` (with `~/.claude/CLAUDE.md`) is the top-level project guide and
**takes precedence** over this file. General lean-ctx integration (native→`ctx_*`
mappings, dev workflow, session continuity, quality bar) lives there — not
repeated here. This doc is subordinate: it only spells out the SDD-specific
coordination + memory contract those guides delegate down and hooks cannot inject.

## NO superpowers SDD bash scripts — use `ctx_session` + `ctx_agent`

CRITICAL: The superpowers `subagent-driven-development` skill ships bash helper
scripts (`scripts/task-brief`, `scripts/review-package`, `.superpowers/sdd/progress.md`).
**Do NOT use them.** They are git/bash artifacts that duplicate — worse — what the
lean-ctx runtime already does natively:

| superpowers SDD script         | lean-ctx replacement (use this)                                          |
|--------------------------------|--------------------------------------------------------------------------|
| `scripts/task-brief PLAN N`    | controller renders the phase via the **CLI** (`lean-md render … --phase task-N`, raw-captured) + warm `ctx_multi_read` — see "Plan brief = CLI phase render" below |
| `scripts/review-package B H`   | reviewer reads the diff via `ctx_read(mode=diff)` / `ctx_shell git diff` |
| `.superpowers/sdd/progress.md` | `ctx_session action=task` (progress) + `ctx_knowledge action=remember`   |
| pasted task/report history     | `ctx_agent action=post` / `action=handoff` (baton), `action=diary`       |

Progress, briefs, reports and review packages move through `ctx_session`,
`ctx_knowledge` and `ctx_agent` — never through the SDD bash scripts or a
git-ignored scratch ledger. **git commits themselves ARE allowed on the working
branch** (the committed code is the deliverable); only the SDD *bash tooling* is
replaced by lean-ctx tools.

## Plan brief = CLI phase render (until `lmd-subagent-driven-development` exists)

Plans are `.lmd.md` with `@phase` isolation — the task brief is the **phase
render**, never a raw slice or a whole-doc read of the plan.

**Render path (temporary):** the native `lmd-subagent-driven-development` skill does
not exist yet, and its intended consumption tool `ctx_md_render` is **not
registered** in this repo's lean-ctx instance (nor is it the default backend — the
CLI is). Until the port lands, the controller produces each brief with the **CLI
renderer**, capturing it **raw**:

    ctx_shell(command="cargo run -q --bin lean-md -- render <plan>.lmd.md --phase task-N --consumer=ai", raw=true)

**`raw=true` is mandatory here — do not stack a second compressor.** lean-md's
render is already the terse, byte-stable (#498) artifact; piping it through the
dense `ctx_shell` default re-compresses and mangles the code an implementer must
write verbatim. This is the `AGENTS.md` "never `ctx_shell raw=true` unless
compression is provably wrong" exception — for code-to-write it *is* provably
wrong.

**Never `ctx_read` a plan `.lmd.md` — render it.** It's a rendered artifact:
`mode=full`/`auto` renders it whole-doc (→ `@import` NotFound cascade, lost
`@phase` isolation), `mode=raw` still renders it (macros consumed). Controller
orientation = render each `--phase`. To read the **raw source** of a `.lmd.md`
you must EDIT (exact edit anchors, Fall B), use `lean-md source <file>` — the
normative `.lmd.md`-access rule lives in the `hard-rules` seed (see it via any
skill render); this file no longer restates it.

> **Single source of truth for tool params/signatures:**
> `docs/reference/appendix-mcp-tools.md` (liegt im lean-ctx-Repo; im lean-md-Repo
> gilt der Addon-Kontrakt `docs/CONTRACT.md`) — human tool map; authoritative
> schemas in `rust/src/tools/registered/<tool>.rs`. Also valid: the auto-generated
> `docs/reference/generated/mcp-tools.md` — but ONLY when freshly generated
> (CI-drift-tested); if in doubt, trust the appendix. Read on demand via
> `ctx_read(path, mode=map|signatures)`. This file carries only the *behavioral*
> contract — never rely on memorized signatures.

## lean-ctx tool set (use these proactively)

Requires `tool_profile = power` (`lean-ctx tools power` → all 72 MCP tools
exposed). Under `power` **every** lean-ctx tool is direct — **call it directly**
(`ctx_read`, `ctx_search`, `ctx_shell`, `ctx_tree`, `ctx_multi_read`, `ctx_delta`,
`ctx_task`, `ctx_handoff`, `ctx_workflow`, `ctx_share`, `ctx_rules`, …). If a tool
shows up **deferred** in an isolated subagent catalog, run
`ToolSearch(query="select:<tool>")` FIRST, then call it directly. **NEVER wrap a
tool in `ctx_call`** (no `ctx_call name=ctx_read`, no `ctx_call name=ctx_task` —
that is pure overhead).

`ctx_call` is now only a **fallback**: use it solely if a tool stays deferred
after `ToolSearch`. (Profiles for reference — `minimal` = 6 tools, `standard` = 22,
`power` = all 72; this contract assumes `power`.)

> **No `ctx_share`:** the lean-ctx file cache is shared across all agents in the
> session (one MCP process). A subagent's first `ctx_read` is already warm, so
> warm-cache push/pull via `ctx_share` is redundant ceremony and is intentionally
> NOT part of this contract. (`ctx_share` from Journey 8 §7 targets the
> **cross-process** case — separate Cursor/Claude/Codex processes — which does not
> apply to subagent-driven-development.) Subagents just `ctx_read` — **never
> `fresh`** (mtime auto-validation keeps cached entries current), **never `raw`**.
> (Subagents never read the plan `.lmd.md` at all — the brief is rendered and
> handed to them; see "Plan brief = CLI phase render".)

| Need                              | Tool                           | Note                                                                                                                                                                                                                                                                          |
|-----------------------------------|--------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Orient at start                   | `ctx_overview` + `ctx_repomap` | repomap = PageRank top symbols                                                                                                                                                                                                                                                |
| Warm-read N files before dispatch | `ctx_multi_read paths=[…]`     | one call, not N× `ctx_read`                                                                                                                                                                                                                                                   |
| Re-read after an edit             | `ctx_delta path=…`             | only changed lines (cheaper than diff)                                                                                                                                                                                                                                        |
| Checkpoint at phase boundary      | `ctx_compress`                 | long-conversation context save                                                                                                                                                                                                                                                |
| Warm cache for a subagent         | (automatic — shared MCP cache) | no `ctx_share`; subagent just `ctx_read`, never `fresh`                                                                                                                                                                                                                       |
| Team coordination / diaries       | `ctx_agent`                    | register/post/read/diary/sync/handoff/share_knowledge                                                                                                                                                                                                                         |
| Blast radius (risk gate)          | `ctx_impact`, `ctx_callgraph`  | standard — direct                                                                                                                                                                                                                                                             |
| A2A task board                    | `ctx_task`                     | actions: create(needs `to_agent`)/update(needs `task_id`+`state`)/list/get/message/cancel/info. State machine: created(implicit)→working→{input-required↔working}→completed\|failed\|canceled (last 3 terminal). NOTE: `in_progress` is NOT valid (08-multi-agent.md §6 typo) |
| Shadow-git of own edits           | `ctx_checkpoint`               | snapshot/log/diff/restore — separate from the user's `.git`; snapshot before+after a change to capture exactly what you modified                                                                                                                                              |
| Rule consistency across agents    | `ctx_rules`                    | sync (distribute rules) / diff (drift) / lint (consistency) / status / init                                                                                                                                                                                                   |

## Controller contract (main agent, drives the plan)

1. **Plan start:** `ctx_overview "<plan-topic>"` + `ctx_repomap` (PageRank top
   symbols); check session restore.
2. Once: `ctx_agent action=register agent_type=claude role=plan`.
3. Persist plan facts twice — durable + team:
    - `ctx_knowledge action=remember category=decision …`
    - `ctx_agent action=post category=decision message="key=val;…"`
4. **Per task, BEFORE dispatch:** warm-read the relevant source files in one call
   via `ctx_multi_read paths=[…]`. The cache is shared across all session agents
   (one MCP process) — the subagent's first `ctx_read` hits these warm entries
   automatically. No `ctx_share` push, no `fresh` needed.
5. Prepend the **Dispatch Contract** (below) to every subagent prompt.
6. **After each task:** `ctx_session action=task value="<task> [N%]"`; durable
   facts via `ctx_knowledge action=remember`.
7. Team status via `ctx_agent action=sync` (not manual polling).
8. **At phase boundaries:** `ctx_compress` to checkpoint the long conversation.

## Implementer subagent contract

1. **Start:** `ctx_agent action=register agent_type=subagent role=dev` (warm cache
   already shared — just `ctx_read`; see the No-`ctx_share` note above).
2. **Tool discipline** is the **Dispatch Contract below** verbatim — that block is
   the single source; don't restate it here.
3. **During work:** `ctx_agent action=diary category=<discovery|decision|blocker|progress|insight>`
   at significant steps.
4. **On finish:** `ctx_agent action=post category=status message="…"` with status
   token (see below) + `ctx_agent action=handoff to_agent=<controller-id>` as baton.
5. Durable gotchas/facts: `ctx_knowledge action=remember`.

## Reviewer subagent contract (spec-reviewer + code-quality-reviewer)

1. **Start:** `ctx_agent action=register agent_type=subagent role=review` (warm
   cache already shared; tool discipline = Dispatch Contract below).
2. Post findings via `ctx_agent action=post category=finding` (in addition to the
   text return to the controller).
3. `ctx_agent action=diary` for non-trivial judgments.

## Dispatch Contract (prepend to EVERY subagent prompt)

> **Contract-Kanon (Single Source, D-7):** Der wörtliche Dispatch-Contract lebt als
> Seed-Datei `content/core/dispatch-contract.lmd.md` (built-in via `include_str!` in
> `src/fragments.rs`, byte-stabil #498) und wird von `@dispatch` automatisch
> vorangestellt. **Nicht duplizieren** — Änderungen erfolgen ausschließlich an der Seed-Datei.
