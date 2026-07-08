@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner (never cargo test)"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# lean-md P0 — GitHub-CI-Release + `[artifacts]` (kein crates.io) — Implementation Plan

## Goal

lean-md über den **`[artifacts]`-Kanal** (GH #725) ausliefern: ein GitHub-CI-Release baut ein
prebuilt CLI-only-Binary, `lean-ctx addon add lean-md` lädt es, verifiziert die SHA-256 und wired
es. **Kein crates.io, kein `cargo publish`, kein standalone `cargo install`.** Sync-Vertrag ist der
Release-Tag `v0.2.0` == `[addon].version` == der Version im Registry-Entry drüben (Branch
**`pr-rebuild`**). Spec: `docs/lean-md/plans/2026-07-07-lmd-dist-p0-crates-publish-design.md` +
`lean-md-next-session.prompt.md` (2026-07-08-Pivot).

## Architecture (ambient repo plumbing — stated once, not per task)

Drei Edit-Surfaces, alle Config/CI — **kein `src/`-Verhaltenscode, kein Render-Output-Change**:
- `lean-ctx-addon.toml` — Authoring-Manifest; bekommt `[artifacts.x86_64-unknown-linux-gnu]`
  (filename/url/sha256), `[addon].version = "0.2.0"`, `min_lean_ctx = "3.9.2"`. **Kein `[install]`.**
- `.github/workflows/release.yml` — neuer Workflow (3 Jobs): `build` (x86_64-linux, `--release
  --locked`, default features = CLI-only) → `release` (GitHub-Release + `SHA256SUMS`) →
  `sync-manifest` (schreibt die echte SHA-256 in den `[artifacts]`-Block zurück, Bot-Commit).
- `Cargo.toml` — `version = "0.2.0"` + `repository`; sonst schlank (kein Publish-Ballast).

**`[artifacts]`-Mechanik** (lean-ctx ist der Installer): `addon add` lädt das Release-Asset in den
managed bin dir (`<data_dir>/addons/bin/lean-md/<version>/`, nie PATH), verifiziert die SHA-256,
pinnt den Spawn-Binhash, wired die Gateway. Trust-Anchor = SHA-256-Pin.

**SHA-Fluss** (grammar-addons-Muster): der `[artifacts]`-Block trägt zunächst einen Platzhalter
(64×`0`); der `sync-manifest`-Job ersetzt ihn nach dem Release durch die echte SHA-256 (Bot-Commit
auf `feat-lmd-v2` — Branch-Push, **kein** Tag → kein Release-Loop).

## Global Constraints

- Non-goal: **kein `src/`/Engine/Renderer-Change** — Config + CI-Workflow only.
- Non-goal: kein crates.io, kein `cargo publish`, kein `[install]`-Block, kein standalone
  `cargo install lean-md`, kein `[[dependencies]]`-Skills (P3), kein weiteres Target (P1).
- Invariant: das prebuilt Binary ist **CLI-only** (`cargo build --release`, default features) →
  kein `mcp`-Feature, kein `lean-ctx-client`-Dep im Release-Build.
- Cross-repo-Vertrag: Release-Tag **`v0.2.0`** == `[addon].version` == Registry-Pin drüben;
  `min_lean_ctx = "3.9.2"`; die SHA-256 im `[artifacts]`-Block == die im Registry-Entry drüben.
- **lean-ctx-seitige Arbeit läuft im Branch `pr-rebuild`** (Registry-Entry `[install]`→`[artifacts]`
  via `gen_registry`-Snapshot) — **Handoff, nicht Teil dieser Session** (anderes Repo).
- **B3 — bekannter Zustand, KEIN Bug:** der Registry-Entry drüben ist bereits live und pinnt noch
  `[install] manager=cargo`; bis Release `v0.2.0` + SHA + Registry-Umstellung steht, ist
  `addon add lean-md` erwartungsgemäß rot (Dangling-Fenster).
- Prerequisite: Task 1 (`[artifacts]`-Block existiert) landet vor Task 2 (der `sync-manifest`-Job
  patcht dessen `sha256`-Slot).
- #498 Determinismus: Config/CI-Edits berühren keinen Render-Output → Determinismus- +
  Fragment-Consistency-Gates bleiben grün — **test gate** (jedes Rot = Regression).

@phase "task-1"
## Task 1: `lean-ctx-addon.toml` `[artifacts]` + `Cargo.toml` version bump

**Files:** modify `lean-ctx-addon.toml`, `Cargo.toml`.
**Interfaces:** `lean-ctx-addon.toml` parst als valides TOML, trägt
`[artifacts.x86_64-unknown-linux-gnu]` (filename/url/sha256-Platzhalter), `[addon].version =
"0.2.0"`, `min_lean_ctx = "3.9.2"`, **kein `[install]`**; `Cargo.toml` `version = "0.2.0"` +
`repository`; `cargo build` (default) grün.

### Step 1 — `Cargo.toml` version + repository

Anchor the `[package]` head: @read Cargo.toml mode=lines:1-9
Apply:

    version = "0.2.0"

Add after the `homepage = …` line:

    repository = "https://github.com/dasTholo/lean-md"

**Do NOT** add crates.io publish keys (`keywords`/`categories`/`readme`) and **do NOT** touch
`[features]` (`default = []` stays — the release binary is CLI-only).

### Step 2 — `lean-ctx-addon.toml`: version + min_lean_ctx

Anchor the `[addon]` metadata: @read lean-ctx-addon.toml mode=lines:1-12
Apply:

    version = "0.2.0"
    min_lean_ctx = "3.9.2"

### Step 3 — add the `[artifacts]` block (new content, verbatim)

Insert a new `[artifacts.x86_64-unknown-linux-gnu]` block **before** the `[mcp]` block. The
`sha256` is a placeholder (64×`0`) — Task 2's `sync-manifest` job writes the real hash post-release:

    [artifacts.x86_64-unknown-linux-gnu]
    filename = "lean-md-x86_64-unknown-linux-gnu.tar.gz"
    url      = "https://github.com/dasTholo/lean-md/releases/download/v0.2.0/lean-md-x86_64-unknown-linux-gnu.tar.gz"
    sha256   = "0000000000000000000000000000000000000000000000000000000000000000"

**Do NOT** add an `[install]` block. Leave `[capabilities].network = "none"` unchanged — with
`[artifacts]`, **lean-ctx** performs the download (not the addon); at runtime lean-md is CLI-only
and only spawns `lean-ctx`, so `none` stays honest. (Verify against `core/addons/audit.rs` drüben
before publish; if audit demands otherwise, that verdict wins.)

### Step 4 — parse + build check

    cargo build

**Expected:** green (default features, CLI-only). Then confirm valid TOML by reading it back:
@read lean-ctx-addon.toml mode=full

**Expected:** `[artifacts.x86_64-unknown-linux-gnu]` present with filename/url/sha256;
`[addon].version = "0.2.0"`; `min_lean_ctx = "3.9.2"`; **no** `[install]` block; no duplicate keys.

### Verify & Close

@call verify("Cargo.toml")
@call verify("lean-ctx-addon.toml")
@call gate("Cargo.toml lean-ctx-addon.toml")
@call commit("Cargo.toml lean-ctx-addon.toml", "feat(dist): [artifacts.x86_64-linux] block + version 0.2.0 (drop crates.io/[install])")
@call remember_decision("lean-md P0 artifacts: lean-ctx-addon.toml gains [artifacts.x86_64-unknown-linux-gnu] (sha256 placeholder), version 0.2.0, min_lean_ctx 3.9.2, no [install], network stays none (lean-ctx does the download). Cargo.toml 0.2.0 + repository, no publish keys, default=[]")
@phase-end

@phase "task-2"
## Task 2: `.github/workflows/release.yml` — build → release → sync-manifest

**Files:** create `.github/workflows/release.yml`.
**Interfaces:** on tag `v[0-9]*`, CI builds `x86_64-unknown-linux-gnu` (release, `--locked`,
CLI-only), publishes a GitHub Release with `lean-md-x86_64-unknown-linux-gnu.tar.gz` + `SHA256SUMS`,
then commits the real SHA-256 into the `[artifacts]` block of `lean-ctx-addon.toml`.

### Step 1 — create the workflow (new content, verbatim)

Action pins mirror the lean-ctx repo (`.github/workflows/release.yml`, verified). Create
`.github/workflows/release.yml`:

    name: Release

    on:
      push:
        tags:
          - 'v[0-9]*'

    permissions:
      contents: read

    jobs:
      build:
        name: Build x86_64-unknown-linux-gnu
        runs-on: ubuntu-22.04
        steps:
          - uses: actions/checkout@v4 # v4
            with:
              persist-credentials: false
          - name: Install Rust toolchain
            uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8 # stable
            with:
              targets: x86_64-unknown-linux-gnu
          - name: Build (CLI-only, default features)
            run: cargo build --release --locked --target x86_64-unknown-linux-gnu
          - name: Package
            run: |
              cd target/x86_64-unknown-linux-gnu/release
              tar czf ../../../lean-md-x86_64-unknown-linux-gnu.tar.gz lean-md
          - name: Upload artifact
            uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
            with:
              name: lean-md-x86_64-unknown-linux-gnu
              path: lean-md-x86_64-unknown-linux-gnu.tar.gz

      release:
        name: Create Release
        needs: build
        runs-on: ubuntu-latest
        permissions:
          contents: write
        steps:
          - uses: actions/checkout@v4 # v4
            with:
              persist-credentials: false
          - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4
            with:
              merge-multiple: true
          - name: Generate checksums
            run: sha256sum lean-md-*.tar.gz > SHA256SUMS
          - name: Create GitHub Release
            uses: softprops/action-gh-release@3bb12739c298aeb8a4eeaf626c5b8d85266b0e65 # v2
            with:
              files: |
                lean-md-*.tar.gz
                SHA256SUMS

      sync-manifest:
        name: Sync [artifacts] sha256 into lean-ctx-addon.toml
        needs: release
        runs-on: ubuntu-latest
        permissions:
          contents: write
        steps:
          - uses: actions/checkout@v4 # v4
            with:
              ref: feat-lmd-v2
          - name: Download checksums
            env:
              GH_TOKEN: ${{ github.token }}
            run: gh release download "${GITHUB_REF_NAME}" --pattern SHA256SUMS --dir .
          - name: Patch [artifacts] sha256
            run: |
              SHA="$(grep 'lean-md-x86_64-unknown-linux-gnu.tar.gz' SHA256SUMS | awk '{print $1}')"
              python3 - "$SHA" <<'PY'
              import re, sys
              sha = sys.argv[1]
              path = "lean-ctx-addon.toml"
              src = open(path).read()
              # Replace the 64-hex sha256 inside [artifacts.x86_64-unknown-linux-gnu] only.
              patched = re.sub(
                  r'(\[artifacts\.x86_64-unknown-linux-gnu\][^\[]*?sha256\s*=\s*")[0-9a-f]{64}(")',
                  lambda m: m.group(1) + sha + m.group(2),
                  src, flags=re.S)
              if patched == src:
                  raise SystemExit("sha256 slot not found or unchanged")
              open(path, "w").write(patched)
              PY
          - name: Commit
            run: |
              git config user.name "github-actions[bot]"
              git config user.email "github-actions[bot]@users.noreply.github.com"
              git add lean-ctx-addon.toml
              git diff --cached --quiet || git commit -m "chore(release): sync [artifacts] sha256 for ${GITHUB_REF_NAME}"
              git push

**Loop-safety:** `sync-manifest` commits to `feat-lmd-v2` (branch push), NOT a tag — the `on:
push: tags` trigger does not re-fire. The `if patched == src` guard fails the job if the sha256
slot is missing (Task 1's placeholder must be in place first).

### Step 2 — local dry-run of the build path (no GitHub needed)

The tag-triggered release + Bot-commit run only on GitHub (out-of-scope). Verify the mechanics the
workflow shells out to, locally:

    rustup target add x86_64-unknown-linux-gnu
    cargo build --release --locked --target x86_64-unknown-linux-gnu

**Expected:** green; the binary exists at `target/x86_64-unknown-linux-gnu/release/lean-md`.

    tar czf /tmp/claude-1000/lean-md-x86_64-unknown-linux-gnu.tar.gz -C target/x86_64-unknown-linux-gnu/release lean-md
    sha256sum /tmp/claude-1000/lean-md-x86_64-unknown-linux-gnu.tar.gz

**Expected:** a `.tar.gz` is produced and a 64-hex SHA-256 prints (this is the value the workflow
would write into `[artifacts]`).

### Step 3 — validate the workflow YAML + the sha256 patch regex

    python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('yaml-ok')"

**Expected:** `yaml-ok` (if PyYAML is absent, run `actionlint .github/workflows/release.yml`
instead, or inspect the three jobs by eye). Then dry-run the patch regex against the manifest with
a dummy hash and confirm it hits the placeholder — **without committing**:

    python3 - <<'PY'
    import re
    src = open("lean-ctx-addon.toml").read()
    dummy = "f"*64
    out = re.sub(r'(\[artifacts\.x86_64-unknown-linux-gnu\][^\[]*?sha256\s*=\s*")[0-9a-f]{64}(")',
                 lambda m: m.group(1)+dummy+m.group(2), src, flags=re.S)
    print("patched" if out != src else "NO-MATCH")
    PY

**Expected:** `patched` — the regex targets exactly the `[artifacts]` sha256 slot. (Discard the
output; do not write it.)

### Verify & Close

@call verify(".github/workflows/release.yml")
@call gate(".github/workflows/release.yml")
@call commit(".github/workflows/release.yml", "ci(release): tag-triggered x86_64-linux build + [artifacts] sha256 sync")
@call remember_decision("lean-md release.yml: 3 jobs (build x86_64-linux --release --locked CLI-only → release with SHA256SUMS → sync-manifest patches [artifacts].sha256 via python-regex, bot-commit to feat-lmd-v2 branch=no tag loop). Action pins mirror lean-ctx repo")
@phase-end

@phase "task-3"
## Task 3: Acceptance — local gates + Registry-Handoff (pr-rebuild) + out-of-scope record

**Files:** none (verification + handoff doc only).
**Interfaces:** full suite + #498 gates green; the lean-ctx-side Registry-Handoff snippet is
recorded; the GitHub-owned remainder (tag push, live `addon add`) is documented as out-of-scope.

### Step 1 — full suite + determinism/fragment gates

@call gate("Cargo.toml lean-ctx-addon.toml .github/workflows/release.yml")

**Expected:** `cargo nextest run` passes; clippy clean; rustfmt clean. The `#498` determinism suite
and the fragment-consistency gate (built-in `include_str!` seed == on-disk seed) stay green —
Config/CI edits touched no render output.

### Step 2 — Registry-Handoff für lean-ctx (Branch `pr-rebuild`)

**Nicht hier gebaut** (anderes Repo). Im lean-ctx-Repo auf Branch **`pr-rebuild`** über die
`gen_registry`-Snapshot-Quelle einpflegen (kein Handedit an `addon_registry.json`; `gen_registry
--check` ist CI-Gate). Den `[install]`-Eintrag durch `[artifacts]` ersetzen — filename/url/sha256
**byte-identisch** zu `lean-ctx-addon.toml` nach dem `sync-manifest`-Commit:

    [artifacts.x86_64-unknown-linux-gnu]
    filename = "lean-md-x86_64-unknown-linux-gnu.tar.gz"
    url      = "https://github.com/dasTholo/lean-md/releases/download/v0.2.0/lean-md-x86_64-unknown-linux-gnu.tar.gz"
    sha256   = "<echte SHA-256 aus dem Release>"

Plus `min_lean_ctx = "3.9.2"`. **Reihenfolge zwingend:** erst Release `v0.2.0` + SHA (Task 2), dann
Registry-Entry drüben umstellen — sonst bleibt `addon add` rot (B3-Dangling-Fenster).

### Step 3 — out-of-scope record (GitHub / Maintainer)

Out-of-scope dieser Session (Maintainer @dasTholo): Tag `v0.2.0` pushen → CI-Release beobachten;
`sync-manifest`-Commit prüfen; `lean-ctx addon add lean-md` auf x86_64-Linux end-to-end (Download →
SHA-Verify → managed bin dir → Wiring → Health); `lean-md mcp` antwortet auf stdio.

@call remember_decision("lean-md P0 artifacts accepted: local gates green, release.yml dry-run (build/tar/sha256) reproduziert, YAML valide. Handoff: Registry-Entry drüben in Branch pr-rebuild von [install] auf [artifacts] umstellen (sha256 == addon.toml). Out-of-scope: tag push, live addon add, mcp smoke (Maintainer/GitHub). B3: addon add rot bis Release+SHA+Umstellung")
@phase-end
