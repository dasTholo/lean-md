# Session-Prompt: lean-md distributionsfähig machen — **GitHub-CI-Release + `[artifacts]`** (kein crates.io)

> **Untracked Hand-off-Datei.** Liegt im lean-ctx-Repo-Root, gehört aber inhaltlich ins
> **lean-md-Repo** (`github.com/dasTholo/lean-md`). In neuer Session dort öffnen bzw.
> Inhalt als Start-Prompt verwenden. Nicht committen.
>
> Erstellt: 2026-07-07 · **Umgebaut 2026-07-08: crates.io verworfen → GitHub-CI/`[artifacts]`** ·
> Gegenstück-Branch in lean-ctx: `pr-rebuild`

---

## Kernentscheidung (2026-07-08) — crates.io ist raus

**lean-md wird ausschließlich über `lean-ctx addon add` konsumiert** (kein standalone
`cargo install lean-md`). Damit ist crates.io reiner Overhead (append-only, compile-at-install,
Publish-Account). Der Binary-Kanal ist stattdessen der **`[artifacts]`-Block** (GH #725):

**lean-ctx ist der Installer.** Bei `addon add lean-md` liest lean-ctx den `[artifacts]`-Block,
lädt das zur Plattform passende **GitHub-Release-Asset**, verifiziert die **SHA-256**, legt das
Binary in seinen **managed bin dir** (`<data_dir>/addons/bin/lean-md/<version>/`, **nie PATH**),
pinnt den Hash als Spawn-Binhash (Tamper → Spawn-Refuse) und wired die Gateway auf den absoluten
Pfad. `addon update` zieht die nächste Version side-by-side, health-checkt, pruned die alte.

∴ **Dein GitHub-Release ist die einzige Binary-Quelle.** Kein cargo, kein crates.io, kein
package-manager, kein `[install]`-Bootstrap.

### Getroffene Parameter (verbindlich, 2026-07-08)

| Frage | Entscheidung |
|---|---|
| **Target-Matrix** | **nur `x86_64-unknown-linux-gnu`** zum Start. Weitere Triples = P1-Ausbau. **Kein `[install]`-cargo-Fallback** → nicht-gebaute Plattformen (noch) nicht installierbar. |
| **Feature-Set des prebuilt Binaries** | **CLI-only** (`cargo build --release`, default features). Das Addon macht Code-Intel outbound über den CLI-Backend → **kein `mcp`-Feature, kein `lean-ctx-client`-Dep im Release-Build** → der crates.io-`path`-Publish-Blocker entfällt ersatzlos. |
| **SHA-256-Fluss** | **`[artifacts]`-Block lebt statisch in `lean-ctx-addon.toml`**; der Release-CI-Job berechnet die SHA-256 und **committet sie zurück** in den Block (self-contained, grammar-addons-Muster). |
| **Version-Pin** | bleibt **`0.2.0`** = Release-Tag **`v0.2.0`** = Sync-Vertrag mit dem Registry-Entry drüben. |

### Rails-Status (lean-ctx-seitig gemergt)

| Rail | Issue | Was es bereitstellt |
|---|---|---|
| `[artifacts.<triple>]` (url+sha256), managed binaries, `addon update` | #725 / PR #729 | **der Binary-Kanal, den wir nutzen** |
| `addon publish --namespace`, hosted installs, `gen_registry`-Snapshots | #726 / PR #734 | P2 (optional) |
| `kind=skills`-Pack + depth-1 Dependency-Resolution | #727 / PR #743 | P3 (Referenzfall lean-md) |

**Reverse-Cut-Disziplin (gilt weiterhin):** Renderer-Engine bleibt out-of-tree — lean-md ist
standalone (`rushdown` + `evalexpr`), lean-ctx hat keine Render-Dependency. lean-ctx bringt
Code-Intel über die stabile `ctx_*`-Wire-Surface; lean-md bringt Plan-Grammatik + Skills. Kein
Engine-Symbol darf zurück nach `lean-ctx/rust/src` lecken.

---

## Ziel dieser Session

lean-md so ausliefern, dass `lean-ctx addon add lean-md` (bzw. `dastholo/lean-md`) **ohne
Side-Loading** funktioniert — über den `[artifacts]`-Kanal, gespeist von einem GitHub-CI-Release.
Die Arbeit zerfällt in **Phase A** (statisches `[artifacts]`-Manifest + Registry drüben) und
**Phase B** (der CI-Release-Workflow, der Release + SHA-Commit automatisiert — **eigene Phase**).

### Phase A — `[artifacts]`-Manifest + `Cargo.toml` schlank + Registry drüben

**1. `lean-ctx-addon.toml` — `[artifacts]` statt `[install]`**
- [ ] `[artifacts.x86_64-unknown-linux-gnu]` deklarieren (SHA zunächst Platzhalter — Phase B füllt ihn):
```toml
[artifacts.x86_64-unknown-linux-gnu]
filename = "lean-md-x86_64-unknown-linux-gnu.tar.gz"
url      = "https://github.com/dasTholo/lean-md/releases/download/v0.2.0/lean-md-x86_64-unknown-linux-gnu.tar.gz"
sha256   = "0000000000000000000000000000000000000000000000000000000000000000"
```
- [ ] `[addon].version = "0.2.0"`, `min_lean_ctx = "3.9.2"` (`[artifacts]` greift ab #725/3.9.2).
- [ ] **`[capabilities].network` prüfen:** beim `[artifacts]`-Weg macht **lean-ctx** den Download
      (nicht das addon); zur Laufzeit ist lean-md CLI-only und spawnt nur `lean-ctx`. → `network`
      kann vermutlich zurück auf **`none`** (der `full` war der cargo-Bootstrap-Vorbehalt).
      Gegen `core/addons/audit.rs` verifizieren, bevor gesetzt.
- [ ] **KEIN `[install]`-Block, KEIN `[[example]]`-Problem** (kein `cargo publish`-Verify mehr).

**2. `Cargo.toml` schlank halten**
- [ ] `version = "0.2.0"`, `repository` ergänzen. **Kein** publish-Ballast
      (`keywords`/`categories`/`readme`/`cargo publish --dry-run` entfallen — wir publishen nicht).
- [ ] `default = []` bleibt (Release-Binary ist CLI-only). `lean-ctx-client` bleibt `path`-Dep
      hinter `mcp` — wird im Release-Build nicht gezogen.

**3. Registry-Entry drüben (`pr-rebuild`)** — via `gen_registry`-Snapshot-Quelle (kein Handedit;
`gen_registry --check` ist CI-Gate). Aktuell pinnt der Entry noch `[install] manager=cargo` → auf
`[artifacts.x86_64-unknown-linux-gnu]` (filename/url/sha256 == Manifest) umstellen, `min_lean_ctx = "3.9.2"`.

### Phase B — GitHub-CI-Release-Workflow (**eigene Phase**, Vorlage: lean-ctx-Workflows)

**Blaupause im lean-ctx-Repo:** `.github/workflows/release.yml` (Build-Matrix, `sha256sum >
SHA256SUMS`, `softprops/action-gh-release`) **und v. a. `.github/workflows/grammar-addons.yml`** —
das baut Plattform-Binaries, lädt sie ins GitHub-Release und **regeneriert eine Registry-JSON mit
`url`+`sha256`-Pins via Bot-Commit/PR**. Genau unser `[artifacts]`-SHA-Rückfluss. Auch relevant:
der `update-homebrew`-Job in `release.yml` (SHA aus `SHA256SUMS` grep → ins Manifest → Bot-Commit).

Neuer Workflow `.github/workflows/release.yml` im **lean-md**-Repo:

- [ ] **Trigger:** `on: push: tags: ['v[0-9]*']` (semver-Release-Tags; siehe lean-ctx-Kommentar,
      der `vscode-v*` o. ä. ausschließt).
- [ ] **`build`-Job**, Matrix zunächst **nur** `x86_64-unknown-linux-gnu` (ubuntu-22.04):
  - `dtolnay/rust-toolchain@stable` mit `targets: x86_64-unknown-linux-gnu`.
  - **`cargo build --release --locked --target x86_64-unknown-linux-gnu`** (default features =
    CLI-only; `--locked` für reproduzierbare Releases — kein Feature-Flag, kein `lean-ctx-client`).
  - Package Unix: `tar czf lean-md-x86_64-unknown-linux-gnu.tar.gz lean-md` (aus
    `target/<triple>/release/`).
  - `actions/upload-artifact`.
- [ ] **`release`-Job** (`needs: build`):
  - `actions/download-artifact` (`merge-multiple: true`).
  - `sha256sum lean-md-*.tar.gz > SHA256SUMS`.
  - `softprops/action-gh-release` mit `files: lean-md-*.tar.gz` + `SHA256SUMS`, `permissions: contents: write`.
- [ ] **`sync-manifest`-Job** (`needs: release`) — der SHA-Rückfluss (grammar-addons-Muster):
  - SHA aus `SHA256SUMS` grep: `grep x86_64-unknown-linux-gnu SHA256SUMS | awk '{print $1}'`.
  - Den Wert in den `sha256`-Slot von `[artifacts.x86_64-unknown-linux-gnu]` in
    `lean-ctx-addon.toml` schreiben (sed/jq), Determinismus: **nur die Hash-Zeile** ändern.
  - Bot-Commit (`github-actions[bot]`) + Push (oder PR wie grammar-addons, falls Branch-Protection).
      **Achtung Loop:** der Commit darf keinen neuen Tag-Release triggern (Tag-Trigger ≠ Push-auf-main).
- [ ] Actions auf **gepinnte SHAs** referenzieren (lean-ctx-Konvention: `uses: …@<sha> # <tag>`),
      `persist-credentials: false` in jedem `checkout`.

### P1 — weitere Target-Triples (Ausbau nach P0)

- [ ] Matrix erweitern (nach `release.yml`-Vorbild): `aarch64-unknown-linux-gnu` (cross,
      `gcc-aarch64-linux-gnu`), `x86_64-apple-darwin`, `aarch64-apple-darwin` (macos-latest), ggf.
      `x86_64-pc-windows-msvc` (`.zip` statt `.tar.gz`, `.exe`-Suffix) und musl-Targets (zigbuild).
- [ ] Pro Triple ein `[artifacts.<triple>]`-Block; der `sync-manifest`-Job schreibt alle SHAs.
- [ ] Resolution: `[artifacts]` → (kein runner, kein `[install]`) → Fehler auf nicht-gebauten
      Plattformen. Breite Abdeckung ohne alle Prebuilds bräuchte einen `[install]`-Fallback (und
      damit doch crates.io) — **bewusst offen gelassen**.

### P2 — Self-Service Publish zu ctxpkg.com (optional, aber empfohlen)
Nutzt #726/#734 — kein curated-Registry-Round-Trip nötig:
```bash
lean-ctx account login && lean-ctx account claim dastholo
lean-ctx addon publish --namespace dastholo --check   # alle Gates offline, kein Upload
lean-ctx addon publish --namespace dastholo           # signieren + hochladen
```
- [ ] `lean-ctx-addon.toml` besteht die Publish-Gates (Schema, runnable `[mcp]`, Description,
      `audit.rs`-Verdict: keine shell-exec/fetch-exec/non-HTTPS/under-declared capabilities).
- [ ] Danach: `lean-ctx addon add dastholo/lean-md` end-to-end (Consent → Preflight → Health-Probe).
- [ ] Maintainer bot Sanity-Check der `[artifacts]`-Hashes an (@yvgude) — Angebot steht in PR #721.

### P3 — `kind=skills`-Pack: Skills aus `include_str!` auslagern (Referenzfall #727)
Heute sind die lmd-Skills (brainstorm, writing-plans, tdd, subagent-driven, executing-plans,
writing-skills, finishing-a-branch) **ins Binary embedded** (`include_str!`). #727 macht daraus
einen separaten, signierten, versionierten Pack.

- [ ] `@dasTholo/lean-md-skills` (kind=skills) bauen:
      `lean-ctx pack create --kind skills --from <skills-dir>` (deterministisch, content-addressed,
      zstd-Blobs, redaction-on-load).
- [ ] In `lean-ctx-addon.toml` als Dependency deklarieren:
      `@dasTholo/lean-md` → `@dasTholo/lean-md-skills: ^1.0` (depth-1 SemVer).
- [ ] `include_str!`-Payload aus dem Binary entfernen → Binary schrumpft; Skills-Updates ohne
      Binary-Release.
- [ ] Verifizieren: `addon add lean-md` installiert **beide**, Lockfile hält das aufgelöste Paar,
      zweiter Install offline-reproduzierbar; Skill-Bodies bleiben über `ctx_md_render` erreichbar.
- [ ] Muster im Addon-Author-Guide dokumentieren („capability + content"-Addon-Blueprint).

### P4 — Publisher-Identität & Signing (Awareness, #728 — blocked/low)
**Kein aktiver Task**, aber beim Publishen mitdenken. Phase 4 (#728, `OPEN`, `status: blocked`,
`priority: low`, gated auf Catalog-Traction) konsolidiert Signing auf **eine** ed25519-Publisher-Identität
über Addon *und* Skills-Pack hinweg, und aktiviert optional paid listings.

- lean-md ist **Apache-2.0/kostenlos** → die Commerce-Aktivierung (`artifact_type=addon`, Stripe/402)
  ist **irrelevant**. Nichts konfigurieren.
- **Eine Publisher-Identität nutzen:** sowohl `@dasTholo/lean-md` (Binary) als auch
  `@dasTholo/lean-md-skills` unter **derselben** `account claim dastholo`-Identität publizieren →
  gemeinsame Reputation/Score über beide Kanäle (das ist der Zweck von #728). Kein Sonderweg.
- Falls jemals ein paid listing: Voraussetzung ist `audit`-pass **und** verified publisher **vor**
  jedem Preis — Security-Gate vor Geld. Für lean-md nicht relevant.

---

## Verifikation P0 (Phase A + B)

- [ ] Tag `v0.2.0` pushen → CI baut `x86_64-linux` (CLI-only), Release erscheint mit
      `lean-md-x86_64-unknown-linux-gnu.tar.gz` + `SHA256SUMS`; `sync-manifest` committet den echten SHA-256.
- [ ] `lean-ctx addon add lean-md` auf x86_64-Linux: Consent → Download → **SHA-256-Verify** →
      managed bin dir → Gateway-Wiring → Health-Probe grün.
- [ ] `lean-md mcp` antwortet auf stdio (`ctx_md_render`/`ctx_md_check`).
- [ ] Reverse-Cut intakt; Determinismus- + Fragment-Consistency-Gates (#498) grün.

### ⚠️ Achtung — bereits live stehender Registry-Entry (B3)

Der lean-ctx-`pr-rebuild`-Registry-Entry ist bereits **live** und pinnt noch
`[install] manager=cargo, version=0.2.0`. Bis Phase A+B stehen (Release `v0.2.0` + echter SHA-256
im `[artifacts]`-Block **und** Registry-Umstellung drüben), schlägt `addon add lean-md` fehl.
**Reihenfolge zwingend: erst Release + SHA, dann Registry-Entry drüben umstellen** — sonst bleibt
`addon add` rot (bekanntes Dangling-Fenster).

---

## Koordination mit lean-ctx `pr-rebuild`

- **Version-Pin ist der Vertrag:** GitHub-Release-Tag `v0.2.0` == `[artifacts]`/`[addon].version`
  hier == Version im Registry-Entry drüben. **Erst Release + SHA, dann drüben umstellen.**
- **`addon_registry.json` ist generiert** (PR #734, `gen_registry`) — der lean-md-Entry **nicht**
  von Hand editieren; über die Snapshot-Quelle + `gen_registry --check` (CI-Gate).
- **Registry-Entry umstellen:** `[install] manager=cargo` → `[artifacts.x86_64-unknown-linux-gnu]`
  (filename/url/sha256 == `lean-ctx-addon.toml`).
- **`min_lean_ctx = 3.9.2`** (die Version mit `[artifacts]`/`kind`-Resolution, #725/#726). Der auf
  PATH installierte `lean-ctx` ist bereits **3.9.2** — kein Reinstall nötig.
- **Skills-Pack (P3)** hebt `min_lean_ctx` ggf. auf die Version mit `kind=skills`-Support (#727).

---

## Referenzen (im lean-ctx-Repo)

- **`.github/workflows/grammar-addons.yml`** — **die** Blaupause: Plattform-Binaries bauen →
  GitHub-Release → Registry-JSON mit url+sha256 via Bot-Commit/PR (unser `[artifacts]`-SHA-Rückfluss).
- **`.github/workflows/release.yml`** — Build-Matrix, `sha256sum > SHA256SUMS`,
  `softprops/action-gh-release`; der `update-homebrew`-Job zeigt SHA-grep → Manifest → Bot-Commit.
- `docs/guides/addons.md` §„Install on add — artifacts, ephemeral runners & the `[install]` block"
  — Resolution-Wege, `[artifacts]`-Schema (filename/url/sha256), managed bin dir.
- `docs/dev/addon-bootstrap-engine.md` — `[install]`-Schema (Fallback-Referenz, hier nicht genutzt).
- `docs/specs/unified-distribution-v1.md` — vollständiges EPIC-Roadmap-Design.
- PR #721 Kommentare (`gh pr view 721 --repo yvgude/lean-ctx`) — Maintainer-Snippets.
- Registry-Schema: `rust/data/addon_registry.json` (aktueller lean-md-Entry als Vorlage).
- Aktueller lean-md-Entry: `min_lean_ctx 3.8.12`, `stdio`, `command "lean-md" args ["mcp"]`,
  `capabilities: network=none, filesystem=read_write, exec=["lean-ctx"]`, **kein `[install]`/`[artifacts]`**.

## Definition of Done (P0)

- GitHub-CI baut `x86_64-linux`-Binary (CLI-only) auf Tag `v0.2.0`; Release trägt `.tar.gz` +
  `SHA256SUMS`; der echte Hash steht im `[artifacts]`-Block von `lean-ctx-addon.toml` (via `sync-manifest`).
- `lean-ctx addon add lean-md` grün (Download → SHA-Verify → managed bin dir → Wiring → Health).
- Registry-Entry drüben auf `[artifacts]` umgestellt, `gen_registry --check` grün.
- Reverse-Cut intakt (kein Engine-Symbol in lean-ctx/rust/src); #498-Gates grün.
- **Nicht** enthalten: crates.io (verworfen), weitere Targets (P1), publish (P2), Skills-Pack (P3).
