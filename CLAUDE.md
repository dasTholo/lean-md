# CLAUDE.md

## Startup

@AGENTS.md

## Project Hard Rules

> lean-ctx tool-discipline (ctx_read/ctx_shell/ctx_search/ctx_tree mapping, read
> modes, CEP, dense output) is loaded globally via `~/.claude/CLAUDE.md`.
> Full project rules: `AGENTS.md` + the vendored addon contract `docs/CONTRACT.md`.
> Not repeated here — only project deltas below.


- **Tests**: always `cargo nextest run`, never `cargo test`
- **Edits**: Non-symbol → `ctx_read(mode=anchored)` → `ctx_patch` (anchored, patch
  by LINE:HASH, never re-emit old text). `ctx_edit` only tiny-span/replace-all;
  symbol rename/move → `ctx_refactor`. Anchored is the default even over native Edit.
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

  > **Nur diese Dev-Umgebung:** ein lokal aus dem lean-ctx-Branch `pr-rebuild` gebautes Binary
  > trägt eine Auto-Render-Delegation — `try_lmd_addon_render`
  > (`rust/src/tools/registered/ctx_read.rs`) bekommt den `mode` gar nicht und rendert jede
  > `.lmd.md`, **auch `mode=raw`**. Sie ist **nie in `upstream/main`** gelangt, also hat kein
  > veröffentlichtes lean-ctx dieses Verhalten. Solange du gegen ein `pr-rebuild`-Binary
  > arbeitest: Roh-Bytes über `lean-md source <file>`. Der Absatz entfällt, sobald PR #721
  > die Delegation entfernt.

## Subagent-Driven Execution

- SDD-Pläne werden mit dem Skill `lmd-subagent-driven-development` ausgeführt — ein
  frischer Implementer-Subagent pro Task, Zwei-Verdikt-Review dazwischen.
- Der Dispatch-Contract lebt als Seed `content/core/dispatch-contract.lmd.md`
  (`include_str!` in `src/fragments.rs`, byte-stabil #498) und wird von `@dispatch`
  automatisch vorangestellt. Nicht duplizieren, nicht von Hand einfügen.
- Fortschritt, Briefs und Batons laufen über `ctx_session`, `ctx_knowledge` und
  `ctx_agent` — nie über Scratch-Dateien.

## Language

- Interaction, chat, plans, specs: **German** with proper umlauts (ä ö ü ß) — never ae / oe / ue / ss
- Code and code comments: **English**
