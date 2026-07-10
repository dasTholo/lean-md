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
- **Rendering lmd-skills (this dev-repo)**: die `SKILL.md`-Stubs zeigen auf das MCP-Tool
  `ctx_md_render`. Es liegt im Gateway-Katalog (dorthin delegiert `ctx_read`), steht dem
  Agenten aber **nicht** als direktes Tool zur Verfügung. Rendere Phasen deshalb **direkt
  über die CLI**; probiere nicht vorher MCP / `ctx_call`:
  `cargo run -q --bin lean-md -- render --skill <skill> --phase <phase> --consumer=ai`
  (Companion statt Phase: `--companion <name>`). **Kein Release-Build** —
  `cargo run` genügt (nach dem ersten Compile gecached).
- **`.lmd.md` lesen**: `ctx_read` liefert **Roh-Source** wie bei jeder anderen Datei;
  Rendern ist explizit und opt-in (CLI oben bzw. `ctx_md_render`).

  > **Übergang (bis PR #721 gemergt ist):** das lokal gebaute lean-ctx (`pr-rebuild`) trägt
  > noch die Auto-Render-Delegation — `try_lmd_addon_render` in
  > `rust/src/tools/registered/ctx_read.rs` bekommt den `mode` gar nicht und rendert jede
  > `.lmd.md`, **auch `mode=raw`**. Solange das so ist: Roh-Bytes ausschließlich über
  > `lean-md source <file>`. Nach dem Merge entfällt dieser Absatz.

## Subagent-Driven Execution

When executing a plan via `superpowers:subagent-driven-development` (one fresh
subagent dispatched per task), the lean-ctx multi-agent + memory contract is
**mandatory** — for the controller and for every dispatched subagent. The
controller MUST prepend the Dispatch Contract to each subagent prompt.



@rules/subagent-multi-agent.md

## Language

- Interaction, chat, plans, specs: **German** with proper umlauts (ä ö ü ß) — never ae / oe / ue / ss
- Code and code comments: **English**
