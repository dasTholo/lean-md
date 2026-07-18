# Dev README — updating `@dastholo/lean-md-skills`

How to ship a change to skill content without releasing a binary.

## Zwei Release-Regime (seit P3, #727)

| Änderung                                     | Kanal  | Ablauf                                                        |
|----------------------------------------------|--------|---------------------------------------------------------------|
| `content/skills/**`                          | Pack   | Bump + `pack create` + `pack publish`. Kein Tag, kein Binary. |
| `content/core/**`, `content/gloss/**`, `content/templates/**`, `src/**` | Binary | Tag `v*` → 5-leg-Build → `sync-manifest` schreibt die SHA-Pins. |

Pack und Binary tragen **unabhängige** SemVer-Linien (initial beide `0.2.0`). Publizierte
Pack-Versionen sind immutable (das Lockfile pinnt `artifact_sha256`), also erzwingt jede
Content-Änderung einen **Pack**-Bump — nie einen Binary-Bump. `version_req = "^0.2"` deckt
`0.2.x` ab; erst ein Sprung auf `0.3.x` verlangt einen Manifest-Bump + Addon-Republish.

### Skill-Content ändern

Die vollständigen, generischen Schritt-für-Schritt-Runbooks — skill-only, binary-only
und binary+pack — leben jetzt in `docs/RELEASING.md`; diese Datei behält nur die
Zwei-Regime-Übersicht.

Der Seed-Bless (`content/core/**`, `content/templates/**`) bleibt der Sonderfall:
`LEAN_MD_BLESS=1 cargo nextest run --test seed_history` **hängt nur an** `content/seeds.sha256`
an, kürzt die Historie nie — Details im Bless-commands-Abschnitt von `docs/RELEASING.md`.

### Lokal ohne Pack entwickeln

`cargo run -- render --skill X --phase Y` greift auf den Debug-Fallback
(`$CARGO_MANIFEST_DIR/content/skills`). Im Release-Binary ist er inert
(`cfg(debug_assertions)`), dort ist ein fehlendes `LEAN_MD_SKILLS_DIR` ein harter Fehler.

## What consumers see

The skills pack is published and live; the addon itself resolves **once its curated registry
entry is listed (PR #721)**. From then on `addon add @dastholo/lean-md` picks the highest
non-yanked version matching `^0.2`, so a fresh install pulls the latest `0.2.x` pack
automatically. For a **skill-only** cut `lean-ctx-addon.toml` is untouched, so the
`kind=addon` pack does **not** need republishing. The **binary+pack** case is different: it
DOES require an `addon publish` after `sync-manifest` (see `docs/RELEASING.md`).

Existing installs are pinned by the lockfile and stay on their resolved version until
`addon update` — that is correct behaviour, not a bug.

## The drift gate

CI builds the pack and asserts its content hash, but never publishes. On a skill-only change
the gate is the only thing that fires: it reports that `content/skills/` no longer matches the
last published pack, which is your reminder to cut `0.2.1`.

The gate verifies. You publish.

## Version coupling

Binary and pack use **independent SemVer**. They start aligned at `0.2.0` purely as a
convenient starting point — that alignment is not a contract and is expected to break.

A content-only fix bumps the pack (say `0.2.x` → `0.2.x+1`) while the binary version holds. That
divergence *is* the benefit of the cut. Only a pack jump to `0.3.x` requires touching
`version_req` in `lean-ctx-addon.toml`, which in turn means republishing the addon pack.
