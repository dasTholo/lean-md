# lean-md

Standalone macro/directive markdown renderer. The render core runs in-process
(rushdown parser + evalexpr expressions); code-intel is outbound via
`backend.call("ctx_*")` — lean-ctx acts as the code-intel backend, not a hard
runtime dependency of the renderer itself.

## What it does

`.lmd.md` is Markdown with directives. The render core (`rushdown` + `evalexpr`)
evaluates macros, conditionals (`@if`/`@consumer`), expressions (`{{ }}`) and
layout fully in-process — a code-intel-free document renders standalone, with no
running lean-ctx. Code-intel directives (`@edit`, `@symbol`, `@refactor`, `@graph`,
…) are dispatched **outbound** to lean-ctx via the `CodeIntelBackend` (CLI default,
MCP opt-in), so lean-md never parses code locally.

## CLI

```sh
lean-md render <file.lmd.md> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]
lean-md check  <file.lmd.md>
lean-md mcp                      # stdio JSON-RPC 2.0 MCP server (lmd_render / lmd_check; alias ctx_md_render / ctx_md_check)
```

- `render` evaluates the document and prints Markdown (`-o` writes to a file).
  `--consumer=human` narrates directives as prose; `--crp` selects the output
  density (token-compressed rendering protocol).
- `check` parse-checks a source and reports header config + directive count.
- `mcp` serves `lmd_render` / `lmd_check` over stdio (alias `ctx_md_render` / `ctx_md_check`)
  — this is the entry point the addon wiring spawns (`command = "lean-md"`, `args = ["mcp"]`).

## Directives (overview)

Render/expression: `@if` / `@consumer`, `{{ expr }}`, `@define` / `@call` / `@import`,
pipes + `@render`. Read/search: `@read`, `@search`, `@list`, `@query`, `@find`,
`@count`, `@env`, `@date`. Code-intel (outbound): `@edit`, `@symbol`, `@refactor`,
`@reformat`, `@inspect`, `@graph`, `@repomap`, `@impact`, `@architecture`, `@outline`,
`@smells`, `@review`, `@routes`. Workflow: `@phase`, `@dispatch`, `@handoff`,
`@remember`, `@recall`.

Full gloss: [`content/gloss/directives.lmd.md`](content/gloss/directives.lmd.md).

## Quickstart

```sh
cat > demo.lmd.md <<'EOF'
@lean-md
consumer: ai

@if consumer == "ai"
Hello {{ consumer }} — this rendered standalone, no lean-ctx needed.
@if-end
EOF
lean-md render demo.lmd.md
```

## Skills

lean-md embeds **9 skills**: 8 native ports of the superpowers process skills — rendered on
demand, one phase at a time (the −88…−95 % token lever) — plus `lmd-rendering-skills`, a
single bootstrap skill that documents the render call itself (not renderable; it has no
body and is a plain SKILL.md, see below). Each
process-skill `SKILL.md` is a delegation stub; the MCP server renders the body/companion via
`lmd_render` (alias `ctx_md_render`) with `skill=<name>` / `phase=<name>` (or
`companion=<name>`) addressing.

| Stage | Skill | Purpose |
|---|---|---|
| Design | `lmd-brainstorm` | Idea → approved design spec |
| Plan | `lmd-writing-plans` | Spec → token-efficient `.lmd.md` plan |
| Execute | `lmd-executing-plans` | Run a plan inline, batch review checkpoints |
| Execute | `lmd-subagent-driven-development` | One implementer subagent per task, two-verdict review |
| Execute | `lmd-dispatching-parallel-agents` | One subagent per independent domain, then conflict-scan |
| Finish | `lmd-finishing-a-development-branch` | Integrate a branch: merge / PR / keep / discard |
| Cross-cutting | `lmd-test-driven-development` | RED→GREEN→REFACTOR before implementation |
| Cross-cutting | `lmd-writing-skills` | TDD for skills — pressure-test first |

Purpose texts are distilled from each skill's `SKILL.md` `description` field, not invented.

### How invocation works

Two levels:

- **End-user entry:** in an agent host (e.g. Claude Code) skills auto-trigger via
  their `description`, or you invoke `/lmd-<skill>` as a slash command; the host
  agent walks the phases.
- **Render mechanics:** every `SKILL.md` is a stub; the body/companion is fetched phase by
  phase through the gateway's `lmd_render` tool (alias `ctx_md_render`) — see "Why
  `ctx_md_render` Is Regularly Misdiagnosed as a Broken Gateway" below for the exact call
  form (a bare `lmd_render(...)` call is not it). CLI equivalent:
  `lean-md render --skill <name> --phase <phase> --consumer=ai`. Phase isolation is where
  the token saving comes from.

### Making the skills discoverable (install the `SKILL.md` stubs)

`addon add` wires the MCP tools and the skill **bodies** (the pack), but it does
**not** place the `SKILL.md` stubs an agent host (e.g. Claude Code) needs to *see*
and auto-trigger the skills. That is why a fresh `addon add` install shows no
skills. Install the stubs with the built-in installer — **local is the default**
(per-repo), `--global` for a user-wide scope:

```sh
lean-md skill install lmd-brainstorm            # → ./.claude/skills/…   (local, default)
lean-md skill install lmd-brainstorm --global   # → ~/.claude/skills/…   (every repo)
lean-md skill remove  lmd-brainstorm [--global] # mirror uninstall
```

Install all nine at once (the canonical `INSTALLABLE_SKILLS` set):

```sh
# A release `lean-md` on your PATH needs the skills pack on the env — the gateway injects
# it only into the MCP process, so a bare shell call fails with PACK_MISSING otherwise.
# (Skip this line if your PATH `lean-md` is a shim that resolves the pack itself.)
export LEAN_MD_SKILLS_DIR="$(find ~/.local/share/lean-ctx/packages/skills/@dastholo__lean-md-skills -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
for s in lmd-brainstorm lmd-writing-plans lmd-executing-plans \
         lmd-subagent-driven-development lmd-dispatching-parallel-agents \
         lmd-finishing-a-development-branch lmd-test-driven-development \
         lmd-writing-skills lmd-rendering-skills; do
  lean-md skill install "$s"          # --local is default; append --global for user-wide
done
```

`lmd-rendering-skills` is listed here only for completeness — every `skill install` call
already pulls it in as a dependency, so installing it explicitly is never required.

Beyond the stub, `install` also materializes any skill assets **and** the project
seeds into `<repo>/.lean-ctx/lean-md/` (dispatch-contract, plan templates — required
by `@dispatch` and the plan recipes). A bare `SKILL.md` symlink would skip those
seeds, so always use the installer. Full flow, the nine skill names, and the
standalone `LEAN_MD_SKILLS_DIR` requirement: [`INSTALL.md`](INSTALL.md).

## Install as a lean-ctx addon

From the registry (recommended) — the `@dastholo/lean-md` addon is **published and
live** on [ctxpkg](https://ctxpkg.com/@dastholo/lean-md); `addon add` pulls the
sha256-pinned prebuilt binary and resolves its skills-pack dependency
[`@dastholo/lean-md-skills`](https://ctxpkg.com/@dastholo/lean-md-skills)
automatically via `[[dependencies]]`. No Rust toolchain required:

```sh
lean-ctx addon add dastholo/lean-md
```

From a local clone (development / from source):

```sh
cargo install --path .
lean-ctx addon add ./lean-ctx-addon.toml
```

> For prerequisites, the full local-build flow, what `addon add` writes, and
> troubleshooting, see [`INSTALL.md`](INSTALL.md).

> **After `addon add`:** restart your MCP client/server so the gateway catalog
> is re-read and the lean-md tools become visible.

### Updating

Upgrade an installed addon to the latest release:

```sh
lean-ctx addon update lean-md
```

This pulls the newest side-by-side binary **and** skills-pack (health-gated, with
automatic prune of the superseded version); restart your MCP client/server
afterwards. Maintainers cutting a release: see [`docs/RELEASING.md`](docs/RELEASING.md).

The gateway aggregates the addon under the **`ctx_tools`** downstream gateway as
`lean-md::lmd_render` / `lean-md::lmd_check` (alias `lean-md::ctx_md_render` /
`lean-md::ctx_md_check`) — **not** on the `ctx_call` / `ctx_discover_tools` router
(those list only lean-ctx's own tools). From an MCP client:

```jsonc
ctx_tools {"action":"list"}     // → lean-md [stdio, enabled] — 4 tool(s)
ctx_tools {"action":"call","tool":"lean-md::lmd_render",
           "arguments":{"path":"demo.lmd.md"}}   // byte-identical to `lean-md render demo.lmd.md`
```

## Why `ctx_md_render` Is Regularly Misdiagnosed as a Broken Gateway

`lean-md` is a separate stdio MCP server; the lean-ctx gateway spawns and aggregates it, but
neither `ctx_md_render(...)` nor its alias `lmd_render(...)` is a directly callable tool — every
call must go through the `ctx_tools` wrapper shown above. A bare, unwrapped call fails, and the
failure reads exactly like "the gateway is down," which sends people straight into the shell
fallback for the rest of the session. The fallback works and reports no error, so the
misdiagnosis conceals itself.

Work through this order before concluding the gateway is broken:

1. `ctx_tools(action="list")` — does it show `lean-md [stdio, enabled]`? Then the server is fine
   and the call was simply misaddressed; fix the call, don't fall back.
2. Got `Transport closed`? Retry once — sporadic, the gateway respawns the server.
3. Only once the server is genuinely absent: fall back to the shell (`lean_md_bin` +
   `LEAN_MD_SKILLS_DIR`, see INSTALL.md's "Standalone requirement") — same binary,
   byte-identical output.

**Step 3 is for consumers of the installed addon.** If you are developing *inside the lean-md
repo itself*, the shell fallback is the *normal* path, not a last resort — that repo's
`CLAUDE.md` documents why: the gateway carries the lean-md catalog, but does not hand the tool
to the dev agent directly, so step 3 is the default entry point there, not an escape hatch.
Reading step 3 in isolation and applying it backwards is the exact failure mode this section
exists to prevent.

`lmd_render` / `lmd_check` are the recommended tool names going forward; `ctx_md_render` /
`ctx_md_check` remain supported aliases with identical behavior. Rendered skill *pack* content
(the `SKILL.md` bodies fetched at runtime) intentionally keeps citing `ctx_md_render` /
`ctx_md_check` — the pack has no lean-md-minimum-version gate to enforce a rename
(`min_lean_ctx` in `lean-ctx-addon.toml` pins the *lean-ctx* version, not lean-md's), so pack
content stays on the name every lean-md version is guaranteed to understand.

The agent-facing counterpart of this section — same diagnosis order, written for an LLM to
follow the moment a render call fails — is the `lmd-rendering-skills` skill.

## Backend selection

lean-md calls lean-ctx code-intel tools via an outbound backend. Two backends
are available:

| Backend | How to select | Notes |
|---------|--------------|-------|
| **CLI** (default) | (no env var needed) | Spawns `lean-ctx call <tool> --project-root <root> --json '<args>'` per call. Stateless; works anywhere `lean-ctx` is in `PATH`. |
| **MCP** (opt-in) | `LEAN_MD_BACKEND=mcp` + `LEAN_MD_MCP_ENDPOINT=<url>` | Warm connection via `lean-ctx-client` against a running lean-ctx MCP/HTTP endpoint. Requires the `mcp` Cargo feature. Falls back to CLI if the endpoint is unreachable or malformed. |

Environment variables (exact names from `src/backend.rs`):

- `LEAN_MD_BACKEND` — set to `mcp` to opt into the MCP backend; any other value
  (or absent) → CLI default.
- `LEAN_MD_MCP_ENDPOINT` — base URL of the lean-ctx MCP/HTTP endpoint
  (e.g. `http://localhost:3100`). Only read when `LEAN_MD_BACKEND=mcp`.

The MCP backend requires `tool_profile = power` on the lean-ctx server side so
all `ctx_*` code-intel tools are exposed.

## Manifest contract

`docs/CONTRACT.md` is the vendored copy of the lean-ctx addon-manifest-v1
specification — the single source of truth for manifest shape, registry shape,
install semantics, security model, and CLI surface. Pinned to
`lean-ctx@2946c165a`. Do not edit it directly; update by re-vendoring from
`lean-ctx/docs/contracts/addon-manifest-v1.md`.

## Acknowledgments

lean-md would not be possible without two projects it stands on:

- **[lean-ctx](https://github.com/yvgude/lean-ctx)** by **yvgude** — the context-engineering runtime that lean-md ships as an addon for, and whose tooling powers its own development workflow.
- **[superpowers](https://github.com/obra/superpowers)** by **obra** — the skill collection whose skills (brainstorming, writing-skills, test-driven-development, and more) lean-md ports as native embedded skills.

It also draws inspiration from kindred work:

- **[markdownai](https://github.com/TheDecipherist/markdownai)** by **TheDecipherist** — a kindred "AI workflow engine" that mixes prose with executable directives to run phase-aware, hallucination-free runbooks; an independent take on the directive-markdown idea lean-md shares.

Our deepest thanks to all of them. Without these projects, this would not exist.
