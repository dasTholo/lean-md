# CLAUDE.md

## Startup

@AGENTS.md

## Project Hard Rules

> lean-ctx tool-discipline (ctx_read/ctx_shell/ctx_search/ctx_tree mapping, read
> modes, CEP, dense output) is loaded globally via `~/.claude/CLAUDE.md`.
> Full project rules: `AGENTS.md` + the vendored addon contract `docs/CONTRACT.md`.
> Not repeated here — only project deltas below.


- **Tests**: always `cargo nextest run`, never `cargo test`
- **Shell — no `&&`/`||`/`;` chaining**: every command is its own invocation.
  Replace `cd <dir> && cargo …` with `cargo … --manifest-path <dir>/Cargo.toml`;
  resolve conditional gates into separate steps with an explicit "Expected:" check.
- **Deferred-tool reflex:** see `~/.claude/CLAUDE.md` Hard Rules — always
  `ToolSearch(query="select:...")` before any Bash workaround.
- **Before `git add`** (per changed file): `cargo fmt` (this is a standalone
  crate; `Cargo.toml` + `src/` live at the repo root).
- **No worktrees** — work directly on the current branch
- **No Brief-/Report-Files**: ctx_session
- **Rendering lmd-skills (this dev-repo)**: the `SKILL.md` stubs point at the MCP
  tool `ctx_md_render`, which is **not registered** in this repo's lean-ctx
  instance — that call fails. Render phases **directly via the CLI**; do NOT probe
  MCP / `ctx_call` first:
  `cargo run -q --bin lean-md -- render --skill <skill> --phase <phase> --consumer=ai`
  (companion instead of phase: `--companion <name>`). **No release build** —
  `cargo run` suffices (cached after the first compile).

## Subagent-Driven Execution

When executing a plan via `superpowers:subagent-driven-development` (one fresh
subagent dispatched per task), the lean-ctx multi-agent + memory contract is
**mandatory** — for the controller and for every dispatched subagent. The
controller MUST prepend the Dispatch Contract to each subagent prompt.



@rules/subagent-multi-agent.md

## Language

- Interaction, chat, plans, specs: **German** with proper umlauts (ä ö ü ß) — never ae / oe / ue / ss
- Code and code comments: **English**
