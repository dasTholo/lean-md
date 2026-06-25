# lean-md

Standalone macro/directive markdown renderer. The render core runs in-process
(rushdown parser + evalexpr expressions); code-intel is outbound via
`backend.call("ctx_*")` — lean-ctx acts as the code-intel backend, not a hard
runtime dependency of the renderer itself.

## Install as a lean-ctx addon

From the registry (once listed):

```sh
lean-ctx addon add lean-md
```

From a local clone:

```sh
lean-ctx addon add ./lean-ctx-addon.toml
```

> **After `addon add`:** restart your MCP client/server so the gateway catalog
> is re-read and the lean-md tools become visible.

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
