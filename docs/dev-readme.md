# Dev README — updating `@dasTholo/lean-md-skills`

How to ship a change to skill content without releasing a binary.

> **Not active yet.** This process describes the world *after* P3 (the
> `include_str!` full-cut, see `docs/lean-md/specs/2026-07-08-lmd-p3-skills-pack-full-cut-design.md`).
> Until P3 lands and its four preconditions are green, skill content is still baked into the
> binary and **every** content change needs a binary release. See "Preconditions" at the
> bottom.

## The two release regimes

After the cut, `content/` splits along one line: what the binary carries, and what the pack
carries. Which one you touched decides what you have to release.

| You changed                                                                | You release                                        |
|----------------------------------------------------------------------------|----------------------------------------------------|
| `content/skills/**` — bodies, companions, assets, skill-local `_includes/` | **pack only** → `@dasTholo/lean-md-skills` `0.2.x` |
| `content/core/**`, `content/gloss/**`                                      | **binary** → tag `v*`, full build, SHA sync        |
| `src/**`                                                                   | **binary**                                         |

The point of the cut is the first row: a typo fix in a skill body must not cost five
cross-compiled binaries. `cargo build --release` produces byte-identical binaries when only
`content/skills/` moved, so the five `[artifacts]` SHA-256 pins in `lean-ctx-addon.toml` stay
valid and `release.yml` has nothing to do.

`content/core/` and `content/gloss/` stay embedded on purpose — they keep the renderer
self-contained for any `.lmd.md`, even when no skill is involved. Changing them changes the
binary.

## Updating skill content

1. **Edit** under `content/skills/<skill>/`.

2. **Test locally.** The dev build reads `content/skills/` directly via the debug fallback —
   no pack needed:

   ```
   cargo nextest run
   cargo run -q --bin lean-md -- render --skill <skill> --phase <phase> --consumer=ai
   ```

3. **Bump the pack version.** Published versions are immutable: the lockfile pins
   `artifact_sha256`, so republishing `0.2.1` with different bytes is refused. Any content
   change needs a new version. Stay inside `0.2.x` — the addon manifest declares
   `version_req = "^0.2"`, which on a `0.x` line means `>=0.2.0, <0.3.0`.

4. **Build the pack** and confirm it is deterministic (build twice, same content hash):

   ```
   lean-ctx pack create --kind skills --from content/skills ...
   ```

   > Verify the exact flags with `lean-ctx pack create --help` — the Rust entry point is
   > `build_skills_pack(dir, name, version, description, author, tags)`. Do not guess the
   > version/output flags from this document.

5. **Publish** to ctxpkg under the `dasTholo` namespace. CI does **not** publish; this is a
   maintainer step, so no publish token lives in the workflow.

   > Verify the command with `lean-ctx pack publish --help` (or `addon publish --help` —
   > confirm which one covers `kind=skills`).

6. **Do not** tag `v*`. No binary release, no `sync-manifest` run.

## What consumers see

`addon add @dasTholo/lean-md` resolves the highest non-yanked version matching `^0.2`, so a
fresh install picks up `0.2.1` automatically. `lean-ctx-addon.toml` is untouched, so the
`kind=addon` pack does **not** need republishing.

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

A content-only fix moves the pack to `0.2.1` while the binary stays at `0.2.0`. That
divergence *is* the benefit of the cut. Only a pack jump to `0.3.x` requires touching
`version_req` in `lean-ctx-addon.toml`, which in turn means republishing the addon pack.

## Preconditions (P3)

None of the above works until all four are green:

| #      | Gate                                                                                                                                                                         |
|--------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **V1** | lean-ctx ships dependency authoring + `{pack_dir:}` env expansion + the `min_lean_ctx` gate (see `lean-ctx/docs/lean-md/specs/2026-07-09-addon-pack-dependencies-design.md`) |
| **V2** | lean-md release `v0.2.0` with real SHA-256 in all five `[artifacts]` blocks                                                                                                  |
| **V3** | curated registry entry addressed (`listed`)                                                                                                                                  |
| **V4** | `@dasTholo/lean-md` **and** `@dasTholo/lean-md-skills` published to the hosted registry                                                                                      |

**V1 is the hard blocker.** Without it, `[[dependencies]]` is silently dropped at publish and
`LEAN_MD_SKILLS_DIR` is never set — a cut binary would fail every `render --skill` at runtime.

The pack cannot ship as a GitHub release asset: the resolver looks up versions only through
the registry index (`core/context_package/deps.rs`). Hosted publishing is mandatory, not
optional.

## Zwei Release-Regime (seit P3, #727)

| Änderung                                     | Kanal  | Ablauf                                                        |
|----------------------------------------------|--------|---------------------------------------------------------------|
| `content/skills/**`                          | Pack   | Bump + `pack create` + `pack publish`. Kein Tag, kein Binary. |
| `content/core/**`, `content/gloss/**`, `src/**` | Binary | Tag `v*` → 5-leg-Build → `sync-manifest` schreibt die SHA-Pins. |

Pack und Binary tragen **unabhängige** SemVer-Linien (initial beide `0.2.0`). Publizierte
Pack-Versionen sind immutable (das Lockfile pinnt `artifact_sha256`), also erzwingt jede
Content-Änderung einen **Pack**-Bump — nie einen Binary-Bump. `version_req = "^0.2"` deckt
`0.2.x` ab; erst ein Sprung auf `0.3.x` verlangt einen Manifest-Bump + Addon-Republish.

### Skill-Content ändern

1. `content/skills/**` editieren.
2. `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` — schreibt `content/skills.sha256`.
3. `lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version <neu> --from content/skills --description "lmd skills"`
4. `content/skills.ctxpkg-hash` aus `<pkg_dir>/manifest.json` (`integrity.content_hash`) aktualisieren.
5. `lean-ctx pack export @dasTholo/lean-md-skills@<neu> --sign --output pack.ctxpkg`
6. `lean-ctx pack publish pack.ctxpkg --token ctxp_…` — **von Hand**. CI verifiziert nur;
   es liegt bewusst kein Publish-Token in der Workflow-Umgebung.

### Lokal ohne Pack entwickeln

`cargo run -- render --skill X --phase Y` greift auf den Debug-Fallback
(`$CARGO_MANIFEST_DIR/content/skills`). Im Release-Binary ist er inert
(`cfg(debug_assertions)`), dort ist ein fehlendes `LEAN_MD_SKILLS_DIR` ein harter Fehler.
