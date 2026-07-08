# P1 — Multi-Triple `[artifacts]` Release (5 Legs, bare binaries)

> **Design-Spec.** Erstellt 2026-07-08. Vorgänger: `lean-md-next-session.prompt.md`
> (Phase A+B abgeschlossen, P0 config+CI ausgeführt `d2a9795..7e9f465`, CI-Rehearsal
> bewiesen `9e278b3`). Branch: `feat-lmd-v2`. Koordination drüben: lean-ctx `pr-rebuild`.

---

## Ziel

`lean-ctx addon add lean-md` funktioniert auf **5 Plattformen** statt einer — über den
`[artifacts]`-Kanal (#725), gespeist vom GitHub-CI-Release. Reine CI-/Manifest-/Registry-Arbeit:
**kein** `src/`- oder Render-Change, Reverse-Cut bleibt intakt (kein Engine-Symbol nach
`lean-ctx/rust/src`).

Der Umbau enthält einen **P0-Fix** (bare-binary statt `.tar.gz`), der ohnehin für **jedes**
funktionierende `addon add` nötig ist — siehe „Kern-Invariante" unten.

## Ziel-Matrix (5 Legs)

| Triple | Runner | Besonderheit |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | Basis (P0), auf bare-binary korrigiert |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04` | cross: `gcc-aarch64-linux-gnu` + Linker-Env |
| `x86_64-apple-darwin` | `macos-latest` | cross auf ARM-Runner via rustup-`targets:` |
| `aarch64-apple-darwin` | `macos-latest` | nativ |
| `x86_64-pc-windows-msvc` | `windows-latest` | nacktes `.exe` (kein Archiv) |

## Kern-Invariante (verifiziert am lean-ctx-Quellcode)

Der `[artifacts]`-Downloader **entpackt nichts**. `fetch_verified`
(`rust/src/core/addons/artifact_install.rs`, fn ~L101) lädt die URL-Bytes, schreibt sie **verbatim**
nach `<managed_bin_dir>/<filename>`, verifiziert den SHA-256 **dieser Bytes**, `chmod 0o555`,
`rename`. `ensure_addon_binary` (L169): `dest = managed_bin_dir.join(filename)`. Es existiert **kein**
tar/gzip/zip-Pfad. Der Doc-Kommentar (`artifact_install.rs` L33) nennt als Beispiel wörtlich
`lean-md-aarch64-apple-darwin` — nackt, ohne Extension; `docs/guides/addons.md` §„Install on add"
zeigt dieselbe Konvention. **Das geladene Asset _ist_ die Binary, Byte für Byte.**

**Konsequenz P0-Fix:** Das aktuelle P0-Manifest liefert `lean-md-x86_64-unknown-linux-gnu.tar.gz`
und pinnt den SHA des **Tarballs**. `addon add` würde den Tarball als `…tar.gz` (0o555) ablegen und
die Gateway `exec`t einen gzip-Blob → Spawn-Fail (der SHA-Check bestünde, die Binary ist trotzdem
unausführbar). Nie aufgefallen, weil das End-to-End-`addon add` nie lief (out-of-scope
Maintainer-Schritt, „expectedly red"). → **Release liefert nackte Binaries**
(`lean-md-<triple>`, Windows `lean-md-x86_64-pc-windows-msvc.exe`), nicht `.tar.gz`/`.zip`. Betrifft
**auch den bestehenden x86_64-Block**.

> **Kein Upstream-Issue.** Code + Doku sind konsistent (bare-binary-only); der `.tar.gz` war der
> lean-md-seitige Fehlschluss. Der Fund wird nur hier im Spec dokumentiert.

## Komponenten & Datenfluss

### ① `build`-Job → Matrix (`.github/workflows/release.yml`)

- `strategy.matrix.include: {target, os, cross?}`, `fail-fast: false`.
- **Dev-Config-Neutralisierung auf JEDEM Leg**: `rm -f .cargo/config.toml rust-toolchain.toml`
  (`shell: bash`). Der mold/nightly-`[profile.dev] codegen-backend`-Gotcha (Commit `624791a`) bricht
  sonst auch die macOS/Windows-Legs — stable cargo erroriert beim Parsen von `[profile.dev]` selbst
  für `--release`. `windows-latest` hat git-bash → `shell: bash` trägt überall.
- `aarch64-unknown-linux-gnu` (`cross: true`, nur ubuntu): `apt-get install gcc-aarch64-linux-gnu` +
  `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc`.
- Beide `*-darwin` cross auf `macos-latest` (ARM-Runner) via rustup-`targets:` — kein macos-13, kein
  Test-Run.
- Build: `cargo build --release --locked --target <triple>` (repo-root, **kein** `rust/`-workdir —
  lean-md ist Root-Crate).
- **Package = rename/copy** der rohen Binary, **kein Archiv**:
  - Unix: `cp target/<triple>/release/lean-md lean-md-<triple>`
  - Windows (`pwsh`): `Copy-Item target/<triple>/release/lean-md.exe lean-md-<triple>.exe`
- `actions/upload-artifact` je Leg (Name = Triple).

### ② `release`-Job (`needs: build`)

- `actions/download-artifact` (`merge-multiple: true`).
- `sha256sum lean-md-* > SHA256SUMS` über die **nackten** Assets (Glob trifft `lean-md-<triple>`
  und `lean-md-<triple>.exe`; `SHA256SUMS` existiert zu dem Zeitpunkt noch nicht).
- `softprops/action-gh-release`: `files: lean-md-*` + `SHA256SUMS`, `permissions: contents: write`.

### ③ `sync-manifest`-Job (`needs: release`) — Ansatz A

Der SHA-Rückfluss, generalisiert vom bestehenden single-block-Regex-Job auf N Blöcke:

- `gh release download <tag> --pattern SHA256SUMS`.
- Python-Loop über die `SHA256SUMS`-Zeilen: pro Zeile `<sha>␣␣<asset>` das Triple ableiten
  (Präfix `lean-md-` strippen, Suffix `.exe` strippen) → block-scoped Regex ersetzt genau den
  64-hex-`sha256`-Slot in `[artifacts.<triple>]`.
- **Hard-Assert: jeder erwartete Block wurde geändert.** Bricht bei Triple-Tippfehler / fehlendem
  Block (Schutz gegen stillen No-Op). Byte-stabil (#498): nur die Hash-Zeichen ändern sich, kein
  Reformat, keine neue Dependency.
- Checkout `ref: feat-lmd-v2`, Bot-Commit (`github-actions[bot]`), Push. **Tag-loop-safe** (Push auf
  Branch ≠ neuer Tag-Trigger).

**Verworfene Alternativen:** (B) TOML-aware (tomlkit) → reserialisiert, Reformat-Risiko bricht das
Determinismus-Gate, neue Dep. (C) ganzen `[artifacts]`-Abschnitt aus der Matrix regenerieren →
invasiv, Reorder-Risiko.

### ④ `lean-ctx-addon.toml`

- 5 `[artifacts.<triple>]`-Blöcke. Je Block:
  - `filename = "lean-md-<triple>"` (Windows: `"lean-md-x86_64-pc-windows-msvc.exe"`) — **bare, keine
    `.tar.gz`**.
  - `url` → `…/releases/download/v0.2.0/<filename>`.
  - `sha256` = Platzhalter `64×0` (Phase-B-Muster; `sync-manifest` füllt echte Werte).
- **Bestehenden x86_64-Block auf bare-binary korrigieren** (P0-Fix: `.tar.gz` raus).
- `[addon].version = "0.2.0"`, `min_lean_ctx = "3.9.2"`, `[capabilities].network = "none"` bleiben.
  **Kein `[install]`-Block.**
- **Kein Bug:** `[mcp].command = "lean-md"` bleibt ≠ `filename = "lean-md-<triple>"`. Die Gateway
  überschreibt `command` beim `add` mit dem absoluten managed-bin-Pfad aus `ensure_addon_binary`
  (`cli/addon_cmd.rs:474`). Nicht „angleichen".

### ⑤ Registry drüben (lean-ctx `pr-rebuild`)

- Alle 5 `[artifacts.<triple>]`-Blöcke via `gen_registry`-Snapshot-Quelle spiegeln (**kein**
  Handedit von `addon_registry.json`; `gen_registry --check` ist CI-Gate).
- Blöcke **byte-identisch** zum Manifest **nach** dem `sync-manifest`-Commit
  (filename/url/sha256), `min_lean_ctx = "3.9.2"`.
- **Reihenfolge-Vertrag (zwingend):** erst Release `v0.2.0` + echte SHAs, **dann** Registry-Flip —
  sonst bleibt `addon add` rot (B3-Dangling-Fenster).

## Error-Handling & Risiken

- **`--locked` über cross-Targets:** aarch64/darwin/windows könnten target-spezifische Lock-Einträge
  brauchen (z. B. `windows-sys`). Erster cross-Build kann einmalig `cargo generate-lockfile`
  erfordern → `Cargo.lock` committen. Lokaler Dry-Run auf x86_64-Linux fängt das nicht.
- **`sync-manifest`-Assert** bricht hart bei nicht-getroffenem Block → kein stiller Placeholder-Ship.
- **Windows-Package-Pfad** getrennt (`pwsh` vs. `bash`) — `if: runner.os == 'Windows'`-Gate wie in
  der lean-ctx-Blaupause.

## Testing / Verifikation (Definition of Done)

1. Tag `v0.2.0` pushen → CI baut alle **5 Legs** (bare-binary, CLI-only, `--release --locked`).
2. Release trägt **5 nackte Assets** (`lean-md-<triple>` / `…-msvc.exe`) + `SHA256SUMS`;
   `sync-manifest` committet **5 echte SHA-256** in `lean-ctx-addon.toml`.
3. `lean-ctx addon add lean-md` **je Plattform** grün: Consent → Download → **SHA-Verify** →
   managed bin dir → Gateway-Wiring → Health-Probe.
4. `lean-md mcp` antwortet auf stdio (`ctx_md_render`/`ctx_md_check`).
5. Registry drüben auf 5 `[artifacts]`-Blöcke umgestellt, `gen_registry --check` grün.
6. Reverse-Cut intakt; Determinismus- + Fragment-Consistency-Gates (#498) grün.

> Schritte 1–4 (Tag-Push, CI-Watch, Live-`addon add`-Smoke) + 5 (Registry-Flip) sind
> **Maintainer-/GitHub-Arbeit** (@dasTholo), wie schon bei P0 out-of-scope für die Coding-Session.
> Diese Session liefert die **Config-/CI-/Manifest-Deltas**.

## Bewusst draußen (YAGNI)

musl-Targets, `[install]`/cargo-Fallback (2026-07-08 gekillt → nicht-gebaute Plattformen erroren
bewusst), per-Target-Smoke-Tests (cross-Bins nicht lauffähig), `x86_64-pc-windows-gnu` +
jemalloc (rein lean-ctx-motiviert), `aarch64-pc-windows-msvc`.

## Betroffene Dateien

- `.github/workflows/release.yml` — Matrix, bare-binary-Package, generalisierter `sync-manifest`.
- `lean-ctx-addon.toml` — 5 `[artifacts]`-Blöcke, x86_64 auf bare-binary korrigiert.
- lean-ctx `pr-rebuild`: Registry-Snapshot-Quelle (anderes Repo, Maintainer-Handoff).
