# Installing lean-md

lean-md is distributed as a lean-ctx **addon**: a standalone MCP server that the
lean-ctx gateway spawns. This guide is the detailed companion to the README's
quick install section.

## Prerequisites

- `lean-ctx >= 3.9.6` on `PATH` (the addon ecosystem + `lean-ctx addon` CLI;
  matches `min_lean_ctx` in `lean-ctx-addon.toml`).
- A Rust toolchain (`cargo`) — **only for Path B** (building from source); Path A pulls a prebuilt binary and needs no toolchain.

## Path A — from the registry (recommended)

The `@dastholo/lean-md` addon is **published and live** on ctxpkg
(<https://ctxpkg.com/@dastholo/lean-md>). This is the recommended install — no Rust
toolchain and no manual pack step: `addon add` fetches the sha256-pinned prebuilt
binary into lean-ctx's managed bin dir and pulls in the skills-pack dependency
`@dastholo/lean-md-skills 0.2.0` automatically via `[[dependencies]]` + `{pack_dir:}`
expansion.

```sh
lean-ctx addon add dastholo/lean-md
```

This resolves directly against the ctxpkg registry — it works today. A curated
*listed* entry for registry discovery/search is tracked separately (PR #721) and is
**not** required for `addon add` to succeed.

## Path B — from a local clone (development / from source)

```sh
cargo install --path .                       # puts `lean-md` on PATH
lean-ctx addon add ./lean-ctx-addon.toml     # wire it into the gateway
```

Use this for local development or building from source. **Note on skills:** the published skills-pack
[`@dastholo/lean-md-skills 0.2.0`](https://ctxpkg.com/@dastholo/lean-md-skills) (on
ctxpkg) is the distribution channel — both it and the addon are now live on ctxpkg. A **debug** build
(`cargo run …`) reads skills straight from `content/skills/` via the debug fallback; a
**release** binary from `cargo install --path .` has no fallback, so `render --skill`
needs `LEAN_MD_SKILLS_DIR` pointed at the pack store (else it fails with `PACK_MISSING`).

## Restart the MCP client/server

**Required after `addon add`:** restart your MCP client/server so the gateway
re-reads its catalog and the lean-md tools become reachable through the
**`ctx_tools`** gateway as `lean-md::ctx_md_render` / `lean-md::ctx_md_check`.
This is the most common "tool not found" cause.

## Install the skill stubs (local default / global)

`addon add` wires the MCP tools and the skill **bodies** (the
`@dastholo/lean-md-skills` pack). It does **not** drop the `SKILL.md` stubs that an
agent host (e.g. Claude Code) reads to discover and auto-trigger the skills — so a
fresh install shows no skills until you install them explicitly:

```sh
lean-md skill install <name>            # local (default) → ./.claude/skills/<name>/
lean-md skill install <name> --global   # global          → ~/.claude/skills/<name>/
lean-md skill remove  <name> [--global] # mirror uninstall (removes the lmd-owned dir)
```

- **Local (default)** — project-relative, env-independent, versionable, team-shareable.
- **Global** — under `claude_state_dir()` = `$CLAUDE_CONFIG_DIR` else `~/.claude`;
  visible in every repo.

Each `install` writes, per invocation:

1. the `SKILL.md` stub into the scope's `skills/<name>/` dir;
2. skill assets (e.g. `lmd-brainstorm` browser scripts, `lmd-writing-skills` graph helper);
3. the **project seeds** into `<cwd>/.lean-ctx/lean-md/` — the dispatch-contract and
   plan templates that `@dispatch` and the plan recipes consume (absent-only; pass
   `--force` / `--refresh` to refresh a stale seed).

> A plain symlink of `SKILL.md` is **not** equivalent — it skips steps 2 and 3, so
> `<cwd>/.lean-ctx/lean-md/` is never created and `@dispatch` has no contract. Use the
> installer.

The eight installable skills:

```sh
for s in lmd-brainstorm lmd-writing-plans lmd-executing-plans \
         lmd-subagent-driven-development lmd-dispatching-parallel-agents \
         lmd-finishing-a-development-branch lmd-test-driven-development \
         lmd-writing-skills; do
  lean-md skill install "$s"          # --local is default; append --global for user-wide
done
```

### Standalone requirement: `LEAN_MD_SKILLS_DIR`

The installer reads each stub through the content cascade:
`<cwd>/.lean-ctx/lean-md/skills/` → `$LEAN_MD_SKILLS_DIR` (the materialized pack) →
`content/skills/` (debug builds only). A **release** `lean-md` invoked from your
shell sees neither the pack env (that is injected only into the gateway-spawned MCP
process) nor the debug fallback — so it fails with `PACK_MISSING` unless you point it
at the pack store first. Resolve the exact path `addon add` recorded, then export it:

```sh
grep LEAN_MD_SKILLS_DIR ~/.config/lean-ctx/config.toml
# e.g. ~/.local/share/lean-ctx/packages/skills/@dastholo__lean-md-skills/<version>
export LEAN_MD_SKILLS_DIR="…/@dastholo__lean-md-skills/<version>"   # <version> = the installed pack (0.2.0)
lean-md skill install lmd-brainstorm --global
```

From a clone you can skip the env entirely with a **debug** build — it reads
`content/skills/` via the debug fallback. `skill install` uses the current dir as its
project root, so run it from the target repo (via `--manifest-path`) to land the stub
and seeds there:

```sh
cargo run --manifest-path /path/to/lean-md/Cargo.toml -- skill install lmd-brainstorm
```

## What `addon add` writes (you maintain nothing by hand)

Installation is automatic and global-only: it upserts a `[[gateway.servers]]`
entry into your global lean-ctx config (`command = "lean-md"`, `args = ["mcp"]`,
plus the declared `[capabilities]`) and records the install in
`<data_dir>/addons/installed.json`. `lean-ctx addon remove lean-md` unwinds both.

## Verify

Direct (no gateway needed):

```sh
lean-md render demo.lmd.md
```

Through the lean-ctx gateway, the addon is aggregated under the **`ctx_tools`**
downstream gateway as `lean-md::ctx_md_render` / `lean-md::ctx_md_check` — **not**
on the `ctx_call` / `ctx_discover_tools` router (those expose only lean-ctx's own
tools). Confirm the wiring and run the round-trip from an MCP client:

```sh
lean-ctx addon list        # → ✓ lean-md … → gateway server `lean-md` (local)
```

```jsonc
ctx_tools {"action":"list"}     // → lean-md [stdio, enabled] — 2 tool(s)
ctx_tools {"action":"call","tool":"lean-md::ctx_md_render",
           "arguments":{"path":"demo.lmd.md"}}
```

Expected: the rendered Markdown of `demo.lmd.md`, byte-identical to
`lean-md render demo.lmd.md`.

## Backend selection (optional — defaults to zero-config CLI)

lean-md is zero-config: by default it shells out to `lean-ctx` per code-intel
directive (`CliBackend`). To use the warm MCP backend instead, set environment
variables (build with the `mcp` feature):

| Variable | Value | Effect |
| --- | --- | --- |
| `LEAN_MD_BACKEND` | `mcp` | opt into the MCP backend (any other value → CLI default) |
| `LEAN_MD_MCP_ENDPOINT` | e.g. `http://localhost:3100` | base URL of a lean-ctx MCP/HTTP endpoint running `tool_profile = power` |

A malformed/unreachable endpoint falls back to the CLI backend — it never bricks
rendering. See the README's "Backend selection" table for details.

## Troubleshooting

- **Tools not visible** → restart the MCP client/server (catalog re-read).
- **`ctx_call` / `ctx_discover_tools` can't find `ctx_md_render`** → expected: that
  router lists only lean-ctx's own tools. Addon tools live on the **`ctx_tools`**
  downstream gateway as `lean-md::ctx_md_render` — invoke via
  `ctx_tools {"action":"call","tool":"lean-md::ctx_md_render", …}`.
- **Tool name carries the `lean-md::` prefix** → that is the gateway namespace
  (`<server>::<tool>`); the prefixed handle is the one to call.
