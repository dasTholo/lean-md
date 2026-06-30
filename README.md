# lean-md

> ⚠️ **Work in progress.** Only the foundation (the render core and addon wiring)
> is in place so far. Creating or executing plans via skills is **not yet
> possible**.

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
lean-md mcp                      # stdio JSON-RPC 2.0 MCP server (ctx_md_render / ctx_md_check)
```

- `render` evaluates the document and prints Markdown (`-o` writes to a file).
  `--consumer=human` narrates directives as prose; `--crp` selects the output
  density (token-compressed rendering protocol).
- `check` parse-checks a source and reports header config + directive count.
- `mcp` serves `ctx_md_render` / `ctx_md_check` over stdio — this is the entry
  point the addon wiring spawns (`command = "lean-md"`, `args = ["mcp"]`).

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

lean-md ships an embedded pilot skill (`content/skills/lmd-brainstorm/`). The MCP
server renders it on demand via `ctx_md_render` with `skill=<name>` / `phase=<name>`
addressing against the binary-embedded body.

## Install as a lean-ctx addon

From the registry (once listed):

```sh
lean-ctx addon add lean-md
```

From a local clone:

```sh
lean-ctx addon add ./lean-ctx-addon.toml
```

> For prerequisites, the full local-build flow, what `addon add` writes, and
> troubleshooting, see [`INSTALL.md`](INSTALL.md).

> **After `addon add`:** restart your MCP client/server so the gateway catalog
> is re-read and the lean-md tools become visible.

The gateway aggregates the addon under the **`ctx_tools`** downstream gateway as
`lean-md::ctx_md_render` / `lean-md::ctx_md_check` — **not** on the `ctx_call` /
`ctx_discover_tools` router (those list only lean-ctx's own tools). From an MCP
client:

```jsonc
ctx_tools {"action":"list"}     // → lean-md [stdio, enabled] — 2 tool(s)
ctx_tools {"action":"call","tool":"lean-md::ctx_md_render",
           "arguments":{"path":"demo.lmd.md"}}   // byte-identical to `lean-md render demo.lmd.md`
```

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

Our deepest thanks to both. Without these repositories, this project would not exist.
