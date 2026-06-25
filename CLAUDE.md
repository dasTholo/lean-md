# CLAUDE.md

## Startup

@AGENTS.md

## Project Hard Rules

> lean-ctx tool-discipline (ctx_read/ctx_shell/ctx_search/ctx_tree mapping, read
> modes, CEP, dense output) is loaded globally via `~/.claude/CLAUDE.md`.
> Full project rules: `AGENTS.md` + the vendored addon contract `docs/CONTRACT.md`.
> Not repeated here — only project deltas below.

- **Tests**: always `cargo nextest run`, never `cargo test`
- **Deferred-tool reflex:** see `~/.claude/CLAUDE.md` Hard Rules — always
  `ToolSearch(query="select:...")` before any Bash workaround.
- **Before `git add`** (per changed file): `cargo fmt` (this is a standalone
  crate; `Cargo.toml` + `src/` live at the repo root).
- **No worktrees** — work directly on the current branch

## Subagent-Driven Execution

When executing a plan via `superpowers:subagent-driven-development` (one fresh
subagent dispatched per task), the lean-ctx multi-agent + memory contract is
**mandatory** — for the controller and for every dispatched subagent. The
controller MUST prepend the Dispatch Contract to each subagent prompt.

@rules/subagent-multi-agent.md

## Language

- Interaction, chat, plans, specs: **German** with proper umlauts (ä ö ü ß) — never ae / oe / ue / ss
- Code and code comments: **English**
