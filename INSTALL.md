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
`@dastholo/lean-md-skills 0.2.1` automatically via `[[dependencies]]` + `{pack_dir:}`
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
[`@dastholo/lean-md-skills 0.2.1`](https://ctxpkg.com/@dastholo/lean-md-skills) (on
ctxpkg) is the distribution channel — both it and the addon are now live on ctxpkg. A **debug** build
(`cargo run …`) reads skills straight from `content/skills/` via the debug fallback; a
**release** binary from `cargo install --path .` has no fallback, so `render --skill`
needs `LEAN_MD_SKILLS_DIR` pointed at the pack store (else it fails with `PACK_MISSING`).

## Restart the MCP client/server

**Required after `addon add`:** restart your MCP client/server so the gateway
re-reads its catalog and the lean-md tools become reachable through the
**`ctx_tools`** gateway as `lean-md::lmd_render` / `lean-md::lmd_check` (alias `ctx_md_render` / `ctx_md_check`).
This is the most common "tool not found" cause.

## Updating

To upgrade an installed addon to the latest release:

```sh
lean-ctx addon update lean-md
```

This fetches the newest side-by-side binary **and** skills-pack from ctxpkg
(health-gated, with automatic prune of the superseded version). Restart the MCP
client/server afterwards so the gateway re-reads its catalog.

Maintainers cutting a release: see [`docs/RELEASING.md`](docs/RELEASING.md) for the
canonical runbook.

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

The nine installable skills:

```sh
# A release `lean-md` needs the skills pack on the env (see "Standalone requirement" below);
# the gateway injects it only into the MCP process, so a bare shell call needs it set here.
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
export LEAN_MD_SKILLS_DIR="…/@dastholo__lean-md-skills/<version>"   # <version> = the installed pack (0.2.1)
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
downstream gateway as `lean-md::lmd_render` / `lean-md::lmd_check` (alias
`lean-md::ctx_md_render` / `lean-md::ctx_md_check`) — **not** on the `ctx_call` /
`ctx_discover_tools` router (those expose only lean-ctx's own tools). Confirm the wiring
and run the round-trip from an MCP client:

```sh
lean-ctx addon list        # → ✓ lean-md … → gateway server `lean-md` (local)
```

```jsonc
ctx_tools {"action":"list"}     // → lean-md [stdio, enabled] — 4 tool(s)
ctx_tools {"action":"call","tool":"lean-md::lmd_render",
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

## Why `ctx_md_render` Is Regularly Misdiagnosed as a Broken Gateway

Neither `ctx_md_render(...)` nor its alias `lmd_render(...)` is a directly callable tool —
lean-md is a separate stdio MCP server, reachable only through the `ctx_tools` gateway
wrapper shown in "Verify" above. A bare, unwrapped call fails, and the failure reads exactly
like "the gateway is down," which sends people into the shell fallback (`LEAN_MD_SKILLS_DIR`,
below) for the rest of the session. The fallback works and reports no error, so the
misdiagnosis conceals itself.

Work through this order before concluding the gateway is broken:

1. `ctx_tools(action="list")` — does it show `lean-md [stdio, enabled]`? Then the server is
   fine and the call was simply misaddressed; fix the call, don't fall back.
2. Got `Transport closed`? Retry once — sporadic, the gateway respawns the server.
3. Only once the server is genuinely absent: fall back to the shell (`lean_md_bin` +
   `LEAN_MD_SKILLS_DIR`, see "Standalone requirement" above) — same binary, byte-identical
   output.

**Step 3 is for consumers of the installed addon.** If you are developing *inside the
lean-md repo itself*, the shell fallback is the *normal* path, not a last resort — that
repo's `CLAUDE.md` documents why: the gateway carries the lean-md catalog, but does not hand
the tool to the dev agent directly, so step 3 is the default entry point there, not an escape
hatch.

`lmd_render` / `lmd_check` are the recommended tool names going forward; `ctx_md_render` /
`ctx_md_check` remain supported aliases with identical behavior. Rendered skill *pack*
content intentionally keeps citing `ctx_md_render` / `ctx_md_check` — the pack has no
lean-md-minimum-version gate to enforce a rename (`min_lean_ctx` pins the *lean-ctx*
version, not lean-md's), so pack content stays on the name every lean-md version is
guaranteed to understand.

The agent-facing counterpart — same diagnosis order, for an LLM to follow the moment a
render call fails — is the `lmd-rendering-skills` skill.

## Troubleshooting

- **Tools not visible** → restart the MCP client/server (catalog re-read).
- **A render call fails and looks like the gateway is down** → see "Why `ctx_md_render` Is
  Regularly Misdiagnosed as a Broken Gateway" above; work the three-step diagnosis before
  falling back to the shell.
- **`ctx_call` / `ctx_discover_tools` can't find `lmd_render` / `ctx_md_render`** → expected:
  that router lists only lean-ctx's own tools. Addon tools live on the **`ctx_tools`**
  downstream gateway as `lean-md::lmd_render` (alias `lean-md::ctx_md_render`) — invoke via
  `ctx_tools {"action":"call","tool":"lean-md::lmd_render", …}`.
- **Tool name carries the `lean-md::` prefix** → that is the gateway namespace
  (`<server>::<tool>`); the prefixed handle is the one to call.
