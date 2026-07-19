# Changelog

All notable changes to lean-md are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

lean-md ships **two independently-versioned release lines** from one repo — the
**binary** (`src/**`, `content/core`, `content/gloss`, `content/templates`,
`Cargo.toml`) and the **skills-pack** (`content/skills/**`). Each carries its own
SemVer; the sections below track them separately.

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
