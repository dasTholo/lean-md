@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo test" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# Release 0.2.1 — Changelog, Docs-Refresh, eindeutiger Addon-Install (Implementation Plan)

Spec: `docs/lean-md/plans/2026-07-18-lmd-release-021-docs-changelog-design.md`.

## Goal

`0.2.1` als Vorbereitungs-Commit auf `feat-lmd-v2` ausliefern: beide Release-Linien
(Binary + Skills-Pack) auf `0.2.1` heben, ein neues `CHANGELOG.md` anlegen, README/INSTALL
aktualisieren (inkl. fehlender Endnutzer-Update-Anleitung), das verstreute Release-Wissen in
ein kanonisches `docs/RELEASING.md` (drei Fälle) konsolidieren und die dev-readme auf die
Regime-Übersicht reduzieren. Danach findet eine neue Session eindeutig `0.2.1`, und der
nächste Release folgt einem geschriebenen Ablauf.

## Architecture

- Zwei entkoppelte SemVer-Linien in einem Repo: **Binary** (`src/**`, `content/core`,
  `content/gloss`, `content/templates`, `Cargo.toml`) als GitHub-Release-Asset
  (`lean-ctx-addon.toml [artifacts.*]` url+sha256); **Skills-Pack** (`content/skills/**`)
  als ctxpkg-Pack `@dastholo/lean-md-skills`, aufgelöst via `[[dependencies]]`.
- `[artifacts.*]` (url/sha256) schreibt **ausschließlich** die Release-CI (`sync-manifest`)
  nach dem Tag-Push — nie von Hand.
- Skills-Hashes leben in `content/skills.sha256` (Drift-Gate `pack_drift`) und
  `content/skills.ctxpkg-hash`; Seed-Historie in `content/seeds.sha256` (append-only,
  `seed_history`).

## Global Constraints

- Non-Goals: kein PR gegen `main`; kein `0.3.x`-Sprung → `version_req = "^0.2"` bleibt
  unangetastet; `[artifacts.*]` url/sha256 **nicht** von Hand ändern (CI-Domäne); kein
  unrelated Refactoring.
- Ausführungsgrenze: der Agent **bereitet vor und committet** auf `feat-lmd-v2`. Kein
  Tag-Push, kein `pack publish`/`addon publish`, kein `ctxp_`-Token im Kontext.
- Determinism/#498: nach dem Skills-Pack-Rebless müssen `pack_drift`, `seed_history`,
  `determinism`, `version_gate` grün bleiben — Test-Gate je Task.
- Prerequisite: `docs/RELEASING.md` (Task 3) landet **vor** Task 4 (dev-readme) und Task 5
  (README/INSTALL) — beide verlinken darauf.
- CHANGELOG-Einträge werden aus den Commit-Betreffen `v0.2.0..HEAD` destilliert, nicht
  erfunden.

@phase "task-1"
## Task 1: Version-Bumps (Binary 0.2.1 + Addon 0.2.1) + Skills-Pack rebless

**Files:** edit `Cargo.toml`, `lean-ctx-addon.toml`; regenerate `Cargo.lock`; rebless
`content/skills.sha256` + `content/skills.ctxpkg-hash`.

**Interfaces / Invariants:**
- `Cargo.toml [package] version = "0.2.1"` (war `0.2.0`).
- `lean-ctx-addon.toml [addon] version = "0.2.1"` (war `0.2.0`).
- `[artifacts.*]` (alle fünf Targets: url + sha256) **unverändert** — CI-Domäne.
- `[[dependencies]] version_req = "^0.2"` **unverändert**.

@call patch("Cargo.toml", "line 3: version = \"0.2.0\" → version = \"0.2.1\"")

@call patch("lean-ctx-addon.toml", "the [addon] table version key (line 4): version = \"0.2.0\" → version = \"0.2.1\". Do NOT touch any [artifacts.*] url/sha256 line and do NOT touch [[dependencies]] version_req.")

Skills-Pack-Hashes gegen `content/skills/` neu blessen (schreibt `content/skills.sha256`
und ggf. `content/skills.ctxpkg-hash`; bei unveränderten Inhalten ein No-op):

    LEAN_MD_BLESS=1 cargo nextest run --test pack_drift

Expected: Test grün. `git status` zeigt entweder keine Skills-Hash-Änderung (Inhalt seit
letztem Bless stabil) **oder** aktualisierte `content/skills.sha256` /
`content/skills.ctxpkg-hash` — beide dann mit committen.

`Cargo.lock` refresht sich beim ersten Build; der `version = "0.2.1"`-Eintrag für `lean-md`
muss danach in `Cargo.lock` stehen.

### Verify & Close

@call verify(Cargo.toml lean-ctx-addon.toml Cargo.lock)
@call gate(Cargo.toml lean-ctx-addon.toml Cargo.lock content/skills.sha256 content/skills.ctxpkg-hash)
@call commit("Cargo.toml lean-ctx-addon.toml Cargo.lock content/skills.sha256 content/skills.ctxpkg-hash", "chore(release): bump binary + addon to 0.2.1, rebless skills pack")
@call remember_decision("0.2.1 prep: Cargo.toml + lean-ctx-addon.toml [addon].version bumped to 0.2.1; [artifacts.*] and version_req deliberately untouched (CI/sync-manifest owns artifacts, ^0.2 still covers 0.2.x).")
@phase-end

@phase "task-2"
## Task 2: `CHANGELOG.md` (neu)

@call recall_context("0.2.1 prep version bumps and what was deliberately untouched")

**Files:** create `CHANGELOG.md` (repo root). Keep-a-Changelog-Format, **eine Datei, zwei
entkoppelte Versionslinien**. Descriptions aus `git log v0.2.0..HEAD` destilliert.

New file (verbatim — does not exist yet):

    # Changelog

    All notable changes to lean-md are documented here. The format is based on
    [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

    lean-md ships **two independently-versioned release lines** from one repo — the
    **binary** (`src/**`, `content/core`, `content/gloss`, `content/templates`,
    `Cargo.toml`) and the **skills-pack** (`content/skills/**`). Each carries its own
    SemVer; the sections below track them separately.

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

@call patch("Cargo.toml", "the [package] include = [...] array: add \"CHANGELOG.md\" so the changelog ships in the crate package (it currently lists README.md and INSTALL.md but not CHANGELOG.md).")

### Verify & Close

@call verify(CHANGELOG.md Cargo.toml)
@call gate(CHANGELOG.md Cargo.toml)
@call commit("CHANGELOG.md Cargo.toml", "docs(changelog): add CHANGELOG.md for binary 0.2.1 + skills-pack 0.2.1")
@phase-end

@phase "task-3"
## Task 3: `docs/RELEASING.md` (neu — kanonisches Runbook, drei Fälle)

**Files:** create `docs/RELEASING.md`. Generisch (`<version>` als Platzhalter); die
Token-/Tag-gebundenen Schritte gehören dem Maintainer.

New file (verbatim — does not exist yet):

    # Releasing lean-md

    lean-md ships **two independently-versioned release lines** from one repo:

    - **Binary** — `src/**`, `content/core/**`, `content/gloss/**`,
      `content/templates/**`, `Cargo.toml`. Delivered as a GitHub-release asset
      (per-target `url` + `sha256`), pinned in `lean-ctx-addon.toml [artifacts.*]`.
    - **Skills-pack** — `content/skills/**`. Delivered as the published ctxpkg pack
      `@dastholo/lean-md-skills`, resolved as an addon `[[dependencies]]`.

    Each line carries its own SemVer; the `0.2.x` alignment is a convenience, not a
    contract. `version_req = "^0.2"` in the addon manifest covers every `0.2.x` pack, so a
    pack bump inside `0.2.x` needs **no** addon-manifest change — only a jump to `0.3.x`
    does.

    Everything below is generic: substitute the concrete version for `<version>` (and the
    tag `v<version>`). Commands that need a publish token or push a tag are the
    maintainer's; an implementing agent stops at the preparation commit.

    ## Which case am I in?

    | Case | Trigger | Core sequence |
    |---|---|---|
    | **Skill-only** | only `content/skills/**` changed | pack bump → `pack create` → `pack export --sign` → `pack publish`. No tag, no binary. |
    | **Binary-only** | `src/**`, `content/core`, `content/gloss`, `content/templates`, `Cargo.toml`/manifest changed | tag `v<version>` → release CI build → `sync-manifest` writes `[artifacts.*]` sha256 → `addon publish`. |
    | **Binary + pack** | both changed (e.g. 0.2.1) | pack bump **and** tag; ordered tag → CI → `sync-manifest` → skills-pack publish → `addon publish`. |

    ## Bless commands (before any publish)

    - Skills-pack drift: `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` — rewrites
      `content/skills.sha256` (and `content/skills.ctxpkg-hash`) to match
      `content/skills/`. Without `LEAN_MD_BLESS` the same test is the CI drift gate.
    - Seed history: `LEAN_MD_BLESS=1 cargo nextest run --test seed_history` — **appends**
      the new seed hash to `content/seeds.sha256`. It never shortens history; a lock-less
      install heals from these lines, so a removed line never heals again.

    ## Case: skill-only

    1. Edit `content/skills/**`.
    2. Bless drift (above). Expected: `content/skills.sha256` updated; `git status` shows it.
    3. `lean-ctx pack create --kind skills --name @dastholo/lean-md-skills --version <version> --from content/skills --description "lmd skills"`.
    4. Sync `content/skills.ctxpkg-hash` from `<pkg_dir>/manifest.json`
       (`integrity.content_hash`).
    5. `lean-ctx pack export @dastholo/lean-md-skills@<version> --sign --output pack.ctxpkg`.
    6. `lean-ctx pack publish pack.ctxpkg --token ctxp_…` — by hand; CI carries no publish
       token.

    No tag, no binary rebuild, `lean-ctx-addon.toml` untouched → the `kind=addon` pack is
    **not** republished.

    ## Case: binary-only

    1. Land the code/seed change on `feat-lmd-v2`; bump `Cargo.toml` `version` (+ `Cargo.lock`).
    2. Bless seed history if `content/core`/`content/templates` changed (above).
    3. `cargo nextest run` green — especially `determinism`, `seed_history`, `version_gate`.
    4. `git tag v<version> && git push --tags` → release CI builds the per-target binaries,
       uploads them as release assets, and `sync-manifest` commits the real `[artifacts.*]`
       `url` + `sha256` onto `feat-lmd-v2`.
    5. `git pull` to fetch the sync-manifest commit.
    6. `lean-ctx addon publish --namespace dastholo` — **after** step 5: the published addon
       pack embeds the `[artifacts.*]` sha256, so publishing before `sync-manifest` would
       pin stale/empty hashes.

    ## Case: binary + pack (the 0.2.1 shape)

    Do both, in this order:

    1. **Preparation commit** on `feat-lmd-v2` (the agent's scope): version bumps, changelog,
       docs, skills rebless. Do **not** touch `[artifacts.*]` by hand.
    2. `git tag v<version> && git push --tags` → release CI → `sync-manifest` commits
       `[artifacts.*]` sha256 on `feat-lmd-v2`.
    3. `git pull`.
    4. Skills-pack: `pack create --version <version>` → verify `content/skills.ctxpkg-hash`
       → `pack export --sign` → `pack publish --token ctxp_…`.
    5. Addon-pack: `lean-ctx addon publish --namespace dastholo`.
    6. Smoke: in a clean context `lean-ctx addon update lean-md` → resolves `<version>` for
       both the binary and the skills-pack.

    ## No target-binary needed

    Neither `addon publish` nor the skills `pack create/export/publish` needs a compiled
    `lean-md` in `target/`. They all run through `lean-ctx`; the binary reaches users as the
    GitHub-release asset (`[artifacts.*]` `url` + `sha256`), not from a local build.
    Expected: `pack create` / `addon publish` succeed on a machine with no `target/lean-md`.

### Verify & Close

@call verify(docs/RELEASING.md)
@call gate(docs/RELEASING.md)
@call commit("docs/RELEASING.md", "docs(spec): add canonical docs/RELEASING.md runbook (skill-only / binary-only / binary+pack)")
@call remember_decision("docs/RELEASING.md is now the canonical release runbook (3 cases). dev-readme (task 4) and README/INSTALL (task 5) link here instead of duplicating steps.")
@phase-end

@phase "task-4"
## Task 4: `docs/dev-readme.md` auf Regime-Übersicht reduzieren

@call recall_context("docs/RELEASING.md canonical runbook exists")

**Files:** edit `docs/dev-readme.md`. Die Zwei-Regime-Tabelle + Versions-Kopplung bleiben;
die Schritt-für-Schritt-Abläufe wandern nach `docs/RELEASING.md` (dedupliziert). Der
bestehende „addon.toml untouched → kein Republish"-Satz wird **explizit auf den
skill-only-Fall eingegrenzt**.

@call patch("docs/dev-readme.md", "Replace the numbered '### Skill-Content ändern' step list (the 6 steps, `content/skills/** editieren` … `pack publish … von Hand`) with a short pointer: the full, generic step-by-step runbooks — skill-only, binary-only, and binary+pack — now live in `docs/RELEASING.md`; this file keeps only the two-regime overview. Keep the note about the seed-history append-only bless as a one-liner cross-referencing RELEASING's Bless-commands section. Do NOT touch the '## Zwei Release-Regime' table, '### Lokal ohne Pack entwickeln', or '## Version coupling'.")

@call patch("docs/dev-readme.md", "In the '## What consumers see' section, scope the claim explicitly to the skill-only case: change the sentence `lean-ctx-addon.toml is untouched, so the kind=addon pack does not need republishing.` to state that this holds for the **skill-only** case; the **binary+pack** case DOES require an `addon publish` after `sync-manifest`, as documented in docs/RELEASING.md.")

### Verify & Close

@call verify(docs/dev-readme.md)
@call gate(docs/dev-readme.md)
@call commit("docs/dev-readme.md", "docs(dev-readme): reduce to regime overview, defer runbooks to docs/RELEASING.md, scope the no-republish claim to skill-only")
@phase-end

@phase "task-5"
## Task 5: `README.md` + `INSTALL.md` — 0.2.1-Bump, Endnutzer-Update, RELEASING-Link

@call recall_context("docs/RELEASING.md canonical runbook exists")

**Files:** edit `README.md`, `INSTALL.md`. Zwei Dateien, ein konzeptioneller Change →
`@call review_change()` als Post-Change-Gate.

**INSTALL.md — `0.2.0` → `0.2.1`** (drei Literal-Stellen; README trägt keine `0.2.0`-Literale):

@call patch("INSTALL.md", "line ~19: `@dastholo/lean-md-skills 0.2.0` → `@dastholo/lean-md-skills 0.2.1` (Path A dependency prose).")
@call patch("INSTALL.md", "line ~38: the linked `@dastholo/lean-md-skills 0.2.0` → `0.2.1` (Path B skills-pack note).")
@call patch("INSTALL.md", "line ~106: the comment `# <version> = the installed pack (0.2.0)` → `(0.2.1)`.")

**Endnutzer-Update-Abschnitt (fehlt heute — neu in beiden Dateien):**

@call patch("README.md", "In the '## Install as a lean-ctx addon' section, after the '**After `addon add`:** restart your MCP client/server' blockquote, add an '### Updating' subsection: `lean-ctx addon update lean-md` pulls the new side-by-side binary AND the new skills-pack (health-gated, auto-prune); restart the MCP client/server afterwards. This is the direct answer to 'how does an end user get the latest version'. Also add a one-line link to `docs/RELEASING.md` for maintainers/contributors near the existing dev-readme/CONTRACT references.")

@call patch("INSTALL.md", "Add an '## Updating' section after '## Restart the MCP client/server': `lean-ctx addon update lean-md` fetches the newest side-by-side binary and skills-pack (health-gated, auto-prune), then restart the MCP client/server. Add a link to `docs/RELEASING.md` as the maintainer release runbook.")

### Verify & Close

@call verify(README.md INSTALL.md)
@call review_change()
@call gate(README.md INSTALL.md)
@call commit("README.md INSTALL.md", "docs: bump skills-pack refs to 0.2.1, add end-user `addon update` section, link docs/RELEASING.md")
@phase-end
