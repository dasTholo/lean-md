# Changelog

All notable changes to lean-md are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

lean-md ships **two independently-versioned release lines** from one repo — the
**binary** (`src/**`, `content/core`, `content/gloss`, `content/templates`,
`Cargo.toml`) and the **skills-pack** (`content/skills/**`). Each carries its own
SemVer; the sections below track them separately.

## [binary 0.2.3] — 2026-08-31

Repairs `@read` and its sibling code-intel anchors on a session-less backend,
and the three ways an MCP-hosted render diverged from the CLI.

**Anchors need lean-ctx ≥ 3.10.1.** `lean-ctx call` only began opening a session
in 3.10.1; below that the degradation path below is what you get. `min_lean_ctx`
deliberately stays at `3.9.6` — the fallback is a feature, not a defect, so an
older lean-ctx is not locked out.

### Added
- Read-only bridges degrade instead of aborting the enclosing phase: a failing
  `@read` now renders a visible note and the phase continues. Write bridges keep
  the abort semantics.
- `@read` falls back to `ctx_outline`, then to a visible self-read order, when
  `ctx_read` answers without a session — the anchors are gone, but the render
  states what it could not resolve instead of silently thinning out.
- `render <file> --phase P` on the CLI, and the matching `phase` argument on the
  MCP `path`/`content` branch.

### Fixed
- MCP `ctx_md_render` ignored `phase` on the `path`/`content` branch and answered
  with a whole-document render; `companion` was swallowed there the same way and
  now errors with `-32602` when no `skill` accompanies it.
- A `@phase` killed every gateway render: the automatic `session_decision` sink
  spawned `lean-ctx call ctx_session` back into the server waiting for the answer
  (`Transport closed`). `lean-md mcp` now silences the automatic phase side
  effects — the per-phase narrative and the fire-and-forget `@on complete` sinks.
  Authored `@remember`/`@handoff`/`@checkpoint`/`@compress` still fire, since
  their backend text IS the rendered output and dropping it would make an MCP
  render disagree with a CLI render of the same source (#498).
- The render jail is the project root (`.lean-ctx/` wins over `.git/`, never above
  `$HOME`), so project-relative `@import` resolves under MCP as it does on the CLI.
- Rendering one isolated phase kept the phase scope; without it a phase's own
  `@on complete` never fired and the renderer printed a false `@on complete
  outside @phase`. Both isolated-phase renders share one wrap now.
- `looks_like_a_deliberate_refusal` no longer treats a substring `outside`/`denied`
  anywhere in tool output as a refusal and reroutes around a perfectly good read.
- The MCP error note interpolated `{e:?}` unsanitized — a `@phase` name containing
  `-->` closed the HTML comment early and spilled into the body.

## [binary 0.2.2] — 2026-07-19

### Fixed
- Re-release to repair the stale **published** addon manifest. The 0.2.1
  `addon publish` shipped with `[artifacts.*].url` still pointing at the v0.2.0
  release asset, so every install pulled a pre-`0a3aebc` binary that lacked
  `lmd-rendering-skills` in `INSTALLABLE_SKILLS` (and its co-install) — `skill
  install lmd-rendering-skills` failed with `unknown installable skill` and no
  other install pulled it in. The in-repo manifest was fixed in `2de0701`/`ffbdc7a`
  but never re-published; this version bump forces clients to refetch it. Binary
  content is byte-identical to 0.2.1 (`git diff v0.2.1..HEAD -- src content/core
  content/templates content/gloss Cargo.toml` is empty).

## [binary 0.2.1] — 2026-07-18

### Added
- Checked-in seed history (`content/seeds.sha256`) with an append-only parser;
  an install without a lock entry now heals its seeds from that history.
- `lean-md.lock` written in `sha256sum` format, recording seed provenance.
- Seed refresh at MCP server start, plus an Ack channel — the user can acknowledge a
  seed conflict (a user-edited seed surfaces as `.new`) instead of being blocked.
- `version_gate`: the skills-pack version span is checked against `ctxpkg.lock`
  (case-insensitive pack-name match); only a span violation warns.
- Declarative `arg_schema` — `check` and the MCP bridge read the same single source.

### Changed
- `.ext` fragment inheritance generalized to every fragment; the dispatch-contract
  special path in `dispatch.rs` was removed.
- MSRV 1.96 → 1.97 (latest stable).
- `sha2` 0.10 → 0.11 (moved into the release profile, `sha256_hex` as single source).
- `regex` 1.12 → 1.13.

### Fixed
- Duplicate `@phase` names break loudly instead of silently swallowing content; a
  fenced `@phase` is treated as documentation, matching the gate.
- `check` returns exit 1 on error and no longer swallows project hints; the lock
  header is English.
- `--list-phases` reports duplicates loudly instead of silently emitting nothing.
- `.new` seed files are written only on absence or divergence, and `ack` reports and
  writes only what actually changed (an unknown flag reports instead of acking all).

## [skills-pack 0.2.1] — 2026-07-18

### Added
- `lmd-rendering-skills` — a bootstrap skill documenting the render call convention;
  pulled in with every `skill install` as a dependency.

### Changed
- The 8 process-skill delegation stubs slimmed and the render handle single-sourced.
- `lmd-test-driven-development` body refreshed; companion edits (bulletproofing,
  testing methodology).
- Gateway claim scoped to the addon topology; bare-call instructions dropped.
