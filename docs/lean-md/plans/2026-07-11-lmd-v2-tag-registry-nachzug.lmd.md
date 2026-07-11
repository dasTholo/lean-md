@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# V2 (Tag v0.2.0 + echte SHA-256) + Registry-Nachzug §5.8 — Implementation Plan

Spec: `docs/lean-md/specs/2026-07-11-lmd-v2-tag-registry-nachzug-design.md`.
Übergeordnet: `docs/lean-md/specs/2026-07-10-lmd-release-path-rev2-design.md` (Gate-Überblick).

## Goal

Gate **V2** schließen (Tag `v0.2.0`, GitHub-Release mit fünf Binaries + `SHA256SUMS`,
fünf echte `[artifacts.*].sha256` in `lean-ctx-addon.toml` auf `feat-lmd-v2`) und den
curated Registry-Entry in `lean-ctx` nachziehen (`min_lean_ctx 3.9.4→3.9.6`,
`version ""→"0.2.0"`).

## Architecture

- Release läuft über CI: `.github/workflows/release.yml` triggert auf Tag `v[0-9]*` →
  5-Leg-Matrix-Build → GH-Release → Job `sync-manifest` patcht die echten SHA-256 in
  `lean-ctx-addon.toml` und bot-committet auf `feat-lmd-v2`.
- Der GitHub-Release ist als **Host** zwingend: die fünf `[artifacts].url` sind hart auf
  `github.com/dasTholo/lean-md/releases/download/v0.2.0/<asset>` verdrahtet. macOS/MSVC
  sind auf einem Linux-Host nicht baubar → CI ist praktikabel-alternativlos.
- Lokales Pre-Flight ist ein **nicht-mutierender Dry-Run** (Temp-Kopie), nie das echte
  Manifest. Einzige Quelle realer SHAs bleibt CIs `sync-manifest`.
- Der Registry-Entry lebt im **anderen Repo** `/home/tholo/Scripts/lean-ctx`
  (`rust/data/addon_registry.json`); Änderung + PR gegen `yvgude/lean-ctx`.

## Global Constraints

- **Fortschrittspflicht (bindend):** Der Fortschritt wird **NICHT pro Task**, sondern
  **einmal beim Plan-Abschluss** (Ende Task 4) in `ctx_knowledge` festgehalten
  (`@call remember_decision(...)`). Der laufende Stand dieser Phase lebt in der
  Knowledge, NICHT in den Spec-Dateien (Rev2-Spec Status-Block). Reviewer prüft: genau
  **ein** Abschluss-Eintrag am Plan-Ende, keine per-Task-Progress-Einträge.
- **Reihenfolge bindend:** Task 1 (Pre-Flight grün) vor Task 2 (Tag); Task 3 (V2-DoD
  verifiziert) vor Task 4 (Registry-Nachzug — an `version = "0.2.0"` gekoppelt).
- Non-goal: keine `release.yml`-Änderung (RC-Weg entfiel); keine SHAs von Hand ins echte
  Manifest schreiben (nur `sync-manifest` schreibt sie).
- Tags sind immutable zu behandeln: bei Leg-Bruch **`v0.2.1`**, nie Force-Retag `v0.2.0`.

@phase "task-1"
## Task 1: Pre-Flight lokal (nicht-mutierend)

Zwei Checks fangen Build-Bruch und Regex-/Triple-Drift ab, bevor ein Tag fällt. **Kein
Commit** — das Dry-Run-Snippet ist ephemer (Scratchpad), nicht eingecheckt (Spec §3).

**Files:** liest `.github/workflows/release.yml` (Patch-Logik) und `lean-ctx-addon.toml`
(read-only); schreibt nur in eine Temp-Kopie.

### 1a — Build-Leg

    cargo build --release --locked --target x86_64-unknown-linux-gnu

**Expected:** Exit 0 (Release-Legs kompilieren mit `--locked`, kein `Cargo.lock`-Drift).

### 1b — Patch-Regex-Dry-Run

Neues Snippet (verbatim, existiert noch nicht — in den Scratchpad schreiben und ausführen):

    #!/usr/bin/env bash
    set -euo pipefail
    BIN=target/x86_64-unknown-linux-gnu/release/lean-md
    TMP=$(mktemp -d)
    cp lean-ctx-addon.toml "$TMP/lean-ctx-addon.toml"
    REAL=$(sha256sum "$BIN" | cut -d' ' -f1)
    DUMMY=1111111111111111111111111111111111111111111111111111111111111111
    cat > "$TMP/SHA256SUMS" <<EOF
    $REAL  lean-md-x86_64-unknown-linux-gnu
    $DUMMY  lean-md-aarch64-unknown-linux-gnu
    $DUMMY  lean-md-x86_64-apple-darwin
    $DUMMY  lean-md-aarch64-apple-darwin
    $DUMMY  lean-md-x86_64-pc-windows-msvc.exe
    EOF
    cd "$TMP"
    python3 - <<'PY'
    import re
    path = "lean-ctx-addon.toml"
    src = open(path).read()
    expected = set(re.findall(r'\[artifacts\.([^\]]+)\]', src))
    patched = src
    seen = set()
    for line in open("SHA256SUMS"):
        line = line.strip()
        if not line:
            continue
        parts = line.split()
        sha, asset = parts[0], parts[-1].lstrip("*")
        triple = asset[len("lean-md-"):]
        if triple.endswith(".exe"):
            triple = triple[: -len(".exe")]
        patched, n = re.subn(
            r'(\[artifacts\.' + re.escape(triple) + r'\][^\[]*?sha256\s*=\s*")[0-9a-f]{64}(")',
            lambda m: m.group(1) + sha + m.group(2),
            patched, flags=re.S)
        if n != 1:
            raise SystemExit(f"block [artifacts.{triple}] not patched (matched {n})")
        seen.add(triple)
    missing = expected - seen
    if missing:
        raise SystemExit(f"no SHA256SUMS entry for: {sorted(missing)}")
    if patched == src:
        raise SystemExit("no sha256 slot changed")
    print("DRY-RUN OK: all 5 [artifacts] blocks patched exactly once")
    PY

**Expected:** `DRY-RUN OK: all 5 [artifacts] blocks patched exactly once`. Kein
`SystemExit` (kein `n != 1`, kein `missing`). Das echte `lean-ctx-addon.toml` bleibt
unberührt (`git status` clean).

@phase-end

@phase "task-2"
## Task 2: Branch pushen + Tag v0.2.0 setzen

**Vorbedingung:** Task 1 grün. `git status` clean (kein ungewollter Working-Tree-Drift).

**Files:** git-Ops only. `feat-lmd-v2` (44 Commits ahead of `origin/feat-lmd-v2`).

### 2a — Branch vorziehen (kein Trigger)

    git push origin feat-lmd-v2

**Expected:** `origin/feat-lmd-v2` steht auf lokalem HEAD; **kein** Workflow-Run
(`release.yml` hängt am Tag, nicht am Branch-Push).

### 2b — Tag setzen und pushen

    git tag v0.2.0
    git push origin v0.2.0

**Expected:** Tag `v0.2.0` auf `origin`; der `Release`-Workflow startet (Actions-Tab).

@phase-end

@phase "task-3"
## Task 3: CI verifizieren — DoD V2

**Vorbedingung:** `release.yml`-Run für `v0.2.0` gestartet. Auf Abschluss der drei Jobs
warten: `build` (5 Legs) → `release` → `sync-manifest`.

**Files:** verifiziert `lean-ctx-addon.toml` (nach `git pull`) + GH-Release-Assets.

### 3a — Bot-Commit holen

    git pull origin feat-lmd-v2

**Expected:** holt den `sync-manifest`-Bot-Commit
(`chore(release): sync [artifacts] sha256 for v0.2.0`).

### 3b — Echte SHAs im Manifest

@call verify(lean-ctx-addon.toml)

**Expected:** die fünf `[artifacts.*].sha256` sind echte 64-hex-SHA-256, **kein** `0000…`.

### 3c — Kreuzprobe gegen SHA256SUMS

    gh release view v0.2.0 --json assets --jq '.assets[].name'

**Expected:** fünf Binaries + `SHA256SUMS` gelistet. Die fünf Manifest-SHAs matchen die
Zeilen in `SHA256SUMS` (Release-Asset herunterladen und vergleichen, falls Zweifel).

**Bei Leg-Bruch:** `release` lief nicht ⇒ keine SHAs, Tag „leer". Fixen, dann `v0.2.1`
(kein Force-Retag). Siehe Spec §7.

@phase-end

@phase "task-4"
## Task 4: Registry-Nachzug §5.8 (Upstream lean-ctx)

**Vorbedingung:** V2 geschlossen (Task 3) — koppelt an `version = "0.2.0"`.

**Files (anderes Repo):** `/home/tholo/Scripts/lean-ctx/rust/data/addon_registry.json`,
Entry `lean-md` (line 411 `min_lean_ctx`, line 393 `version`). `verified = false` bleibt.

### 4a — Entry anpassen

@call patch("/home/tholo/Scripts/lean-ctx/rust/data/addon_registry.json", "lean-md-Entry: min_lean_ctx 3.9.4→3.9.6 (line 411) und version ''→'0.2.0' (line 393); verified unverändert false")

**Expected (Diff):** genau zwei geänderte Zeilen im `lean-md`-Entry —
`"min_lean_ctx": "3.9.6"` und `"version": "0.2.0"`. Kein anderer Entry berührt.

### 4b — Commit + PR im lean-ctx-Repo

    git -C /home/tholo/Scripts/lean-ctx add rust/data/addon_registry.json
    git -C /home/tholo/Scripts/lean-ctx commit -m "chore(registry): lean-md min_lean_ctx 3.9.6 + version 0.2.0"
    git -C /home/tholo/Scripts/lean-ctx push
    gh pr create --repo yvgude/lean-ctx --fill

**Expected:** PR gegen `yvgude/lean-ctx` offen; der `lean-md`-Entry behauptet denselben
`3.9.6`-Vertrag wie lean-mds Manifest und trägt die Release-Version.

### Plan-Abschluss (einziger Knowledge-Eintrag der Phase)

@call remember_decision("Phase V2 + §5.8 ABGESCHLOSSEN. V2: Tag v0.2.0 released, 5 Binaries + SHA256SUMS im GH-Release, sync-manifest hat fünf echte SHA-256 in lean-ctx-addon.toml auf feat-lmd-v2 geschrieben (kein 0000…), matchend zu SHA256SUMS — Rev2 §2: V2 ✅. §5.8: Registry-PR gegen yvgude/lean-ctx offen (lean-md-Entry min_lean_ctx 3.9.6 + version 0.2.0, verified false). Verbleibend (Rev2 §5): V4a Pack publish §5.4, V4b addon publish §5.7, Smoke Teil 2 §5.5, Voll-Smoke §5.9.")

@phase-end
