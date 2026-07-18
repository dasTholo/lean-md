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

| Case              | Trigger                                                                                       | Core sequence                                                                                          |
|-------------------|-----------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| **Skill-only**    | only `content/skills/**` changed                                                              | pack bump → `pack create` → `pack export --sign` → `pack publish`. No tag, no binary.                  |
| **Binary-only**   | `src/**`, `content/core`, `content/gloss`, `content/templates`, `Cargo.toml`/manifest changed | tag `v<version>` → release CI build → `sync-manifest` writes `[artifacts.*]` sha256 → `addon publish`. |
| **Binary + pack** | both changed (e.g. 0.2.1)                                                                     | pack bump **and** tag; ordered tag → CI → `sync-manifest` → skills-pack publish → `addon publish`.     |

## Bless commands (before any publish)

- Skills-pack drift: `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` — rewrites
  **only** `content/skills.sha256` to match `content/skills/`. Without `LEAN_MD_BLESS`
  the same test is the CI drift gate. It does **not** write
  `content/skills.ctxpkg-hash`: that is lean-ctx's compressed `content_hash`, produced
  by `pack create`, and **no test regenerates it**. Whenever `content/skills/` changes
  you must copy it by hand from the freshly built pack's `manifest.json`
  (`integrity.content_hash`) — otherwise the `pack-drift.yml` `ctxpkg-hash` cross-check
  goes red against the `min_lean_ctx` binary (exactly how 0.2.1 shipped a stale hash).
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
   docs, skills rebless. **Skills rebless is two hashes, not one:** the `pack_drift` bless
   writes `content/skills.sha256`, and you must additionally copy
   `content/skills.ctxpkg-hash` from a local `pack create`'s `manifest.json`
   (`integrity.content_hash`) — the bless never touches it, and a stale value turns
   `pack-drift.yml` red on the very first push (the 0.2.1 miss). Do **not** touch
   `[artifacts.*]` by hand.
2. `git tag v<version> && git push --tags` → release CI → `sync-manifest` commits
   `[artifacts.*]` sha256 on `feat-lmd-v2`.
3. `git pull`.
4. Skills-pack: `pack create --version <version>` → verify `content/skills.ctxpkg-hash`
   equals this pack's `manifest.json` `integrity.content_hash` (already synced in step 1;
   this is the confirming check, not the first write) → `pack export --sign` →
   `pack publish --token ctxp_…`.
5. Addon-pack: `lean-ctx addon publish --namespace dastholo`.
6. Smoke: in a clean context `lean-ctx addon update lean-md` → resolves `<version>` for
   both the binary and the skills-pack.

## No target-binary needed

Neither `addon publish` nor the skills `pack create/export/publish` needs a compiled
`lean-md` in `target/`. They all run through `lean-ctx`; the binary reaches users as the
GitHub-release asset (`[artifacts.*]` `url` + `sha256`), not from a local build.
Expected: `pack create` / `addon publish` succeed on a machine with no `target/lean-md`.
