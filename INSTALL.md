# Installing lean-md

lean-md is distributed as a lean-ctx **addon**: a standalone MCP server that the
lean-ctx gateway spawns. This guide is the detailed companion to the README's
quick install section.

## Prerequisites

- `lean-ctx >= 3.8.13` on `PATH` (the addon ecosystem + `lean-ctx addon` CLI).
- A Rust toolchain (`cargo`) to build/install from source.

## Path A — from the registry (once listed)

```sh
lean-ctx addon add lean-md
```

## Path B — from a local clone

```sh
cargo install --path .                       # puts `lean-md` on PATH
lean-ctx addon add ./lean-ctx-addon.toml     # wire it into the gateway
```

## Restart the MCP client/server

**Required after `addon add`:** restart your MCP client/server so the gateway
re-reads its catalog and the lean-md tools (`ctx_md_render` / `ctx_md_check`)
become visible. This is the most common "tool not found" cause.

## What `addon add` writes (you maintain nothing by hand)

Installation is automatic and global-only: it upserts a `[[gateway.servers]]`
entry into your global lean-ctx config (`command = "lean-md"`, `args = ["mcp"]`,
plus the declared `[capabilities]`) and records the install in
`<data_dir>/addons/installed.json`. `lean-ctx addon remove lean-md` unwinds both.

## Verify

```sh
lean-ctx call ctx_md_render --project-root . --json '{"path": "demo.lmd.md"}'
```

Expected: the rendered Markdown of `demo.lmd.md`, identical to `lean-md render demo.lmd.md`.

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
- **Tool name has a `lean-md::` prefix** → see the gateway namespacing note in the
  README; the addon path still works, only the visible tool name differs.
