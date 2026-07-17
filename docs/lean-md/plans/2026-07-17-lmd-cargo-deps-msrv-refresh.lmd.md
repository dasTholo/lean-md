@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo test" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# Cargo-Deps + MSRV Refresh — Implementation Plan

Spec: `docs/lean-md/specs/2026-07-17-cargo-deps-msrv-refresh-design.md`

## Goal

Drei Versionszeilen in `Cargo.toml` auf den aktuellen Stand ziehen — `regex 1.12 → 1.13`,
`rust-version 1.96 → 1.97` (letzte Stable), `sha2 0.10 → 0.11` (dev-dep) — bei durchgehend
`major.minor`-Format. Jeder Bump ist ein eigener Commit mit eigenem Verifikations-Gate.

## Architecture

`Cargo.toml` ist die einzige Quelle der MSRV — `rust-toolchain.toml` pinnt nightly als
dev-only Cranelift-Accelerator, CI neutralisiert ihn (`rm -f`) und baut auf Stable. Daraus
folgt: ein lokaler `cargo check` läuft gegen nightly 1.99 und prüft die MSRV-Behauptung
**nicht** — nur `rustup run stable cargo check` tut das.

`sha2` hat genau einen Konsumenten: `tests/pack_drift.rs`, Fn `render_manifest` (`Sha256::new`
/ `update` / `finalize`). Dieser Test ist zugleich das Gate, das `content/skills.sha256`
byte-stabil hält.

## Global Constraints

- Format-Invariante: jede Version in `Cargo.toml` bleibt `major.minor` — nie `major.minor.patch`.
- `content/skills.sha256` bleibt byte-identisch; `pack_drift` läuft **ohne** `LEAN_MD_BLESS=1`
  grün (#498) — Test-Gate, kein Prosa-Versprechen.
- Non-goal: kein dauerhaftes Format-Gate (Test/CI); kein `cargo update` der transitiven Deps
  (`Cargo.lock` bewegt sich nur, soweit die drei Bumps es erzwingen); kein Anfassen von
  `rust-toolchain.toml`.
- Non-goal: die übrigen Deps (`rushdown 0.18`, `evalexpr 13.1`, `serde_json 1.0`, `chrono 0.4`,
  `lean-ctx-client 0.1`, `tiktoken-rs 0.12`) sind bereits aktuell und bleiben unangetastet.
- Reihenfolge bindend: Task 1 → Task 2 → Task 3, je ein eigener Commit. Ein roter Lauf muss
  ohne Bisect zuordenbar bleiben.
- MSRV 1.97 sperrt crates.io-Konsumenten auf 1.96 aus — bewusst akzeptiert (v0.2.0).

@phase "task-1"
## Task 1: regex 1.12 → 1.13

**Files:** `Cargo.toml` (Zeile 24, `[dependencies]`).

Reiner Minor-Bump innerhalb `1.x` — semver-kompatibel, keine Codeänderung erwartet.

@call patch("Cargo.toml", "die [dependencies]-Zeile regex von 1.12 auf 1.13 (Wert in Quotes)")

**Expected:** `Cargo.toml` enthält `regex = "1.13"`; kein Patch-Level in der Zeile.

`Cargo.lock` zieht `regex` beim nächsten Build auf 1.13.x nach. Prüfen:

    cargo tree --depth 1 --edges normal,build

**Expected:** `regex v1.13.x` (nicht 1.12.x); alle übrigen Zeilen unverändert
(`chrono v0.4.45`, `evalexpr v13.1.0`, `rushdown v0.18.0`, `serde_json v1.0.150`).

### Verify & Close

@call verify(Cargo.toml)
@call gate(Cargo.toml)
@call commit("Cargo.toml Cargo.lock", "chore(deps): regex 1.12 -> 1.13")

@phase-end

@phase "task-2"
## Task 2: rust-version 1.96 → 1.97

**Files:** `Cargo.toml` (Zeile 5, `[package]`).

@call patch("Cargo.toml", "die [package]-Zeile rust-version von 1.96 auf 1.97 (Wert in Quotes)")

**Expected:** `Cargo.toml:5` lautet `rust-version = "1.97"`.

### MSRV-Verifikation (der Knackpunkt dieses Tasks)

Der lokale Default ist nightly 1.99 (`rust-toolchain.toml` pinnt nightly für das
Cranelift-Backend). Ein normaler `cargo check` würde die MSRV-Behauptung **nicht** prüfen —
er liefe gegen nightly. Nur der explizite Stable-Lauf beweist, dass 1.97 trägt:

    rustup run stable rustc --version

**Expected:** `rustc 1.97.x` — bestätigt, dass Stable der MSRV entspricht. Meldet es 1.96
oder älter, zuerst `rustup update stable`; meldet es 1.98+, ist die Spec-Annahme veraltet →
`rust-version` auf die tatsächlich gemeldete Stable setzen und diesen Task neu verifizieren.

    rustup run stable cargo check --all-targets

**Expected:** exit 0, keine Fehler. Genau dieser Lauf ist das Gate des Tasks — schlägt er
fehl, nutzt eine Dependency ein Feature jenseits von 1.97 und die MSRV muss höher.

Das Standard-`gate` läuft gegen nightly und ersetzt den Stable-Check **nicht** — beide laufen.

### Verify & Close

@call verify(Cargo.toml)
@call gate(Cargo.toml)
@call commit("Cargo.toml", "chore(deps): rust-version 1.96 -> 1.97 (latest stable)")

@phase-end

@phase "task-3"
## Task 3: sha2 0.10 → 0.11 (dev-dep)

**Files:** `Cargo.toml` (Zeile 37, `[dev-dependencies]`), `tests/pack_drift.rs`.

Breaking Release. Einziger Konsument im gesamten Crate ist die Fn `render_manifest` in
`tests/pack_drift.rs:57-76` — dort zuerst `@read tests/pack_drift.rs mode=anchored` für den
aktuellen Stand. Relevant sind genau drei Aufrufe: `Sha256::new()` (`:71`), `h.update(&bytes)`
(`:72`) und das `{:x}`-Formatting von `h.finalize()` (`:73`). Der Import steht in
`tests/pack_drift.rs:17` (`use sha2::{Digest, Sha256};`). Sonst nutzt kein Code im Crate `sha2`.

@call patch("Cargo.toml", "die [dev-dependencies]-Zeile sha2 von 0.10 auf 0.11 (Wert in Quotes)")

**Expected:** `Cargo.toml` enthält `sha2 = "0.11"` unter `[dev-dependencies]`.

### Erwarteter Bruch und Fallback

In 0.11 liefert `finalize()` ein `hybrid_array::Array` statt `GenericArray`. Implementiert das
kein `LowerHex`, bricht Zeile 73 (`format!("{:x}  {rel}\n", h.finalize())`) mit
`the trait bound ... LowerHex is not satisfied`. Dann Zeile 73 durch einen Hex-Encoder über
die Bytes ersetzen — neuer Code, verbatim:

    let digest = h.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    out.push_str(&format!("{hex}  {rel}\n"));

Kompiliert `{:x}` weiterhin, bleibt Zeile 73 unverändert — dann keinen Fallback einbauen
(YAGNI). Erst den Testlauf entscheiden lassen:

    cargo nextest run --test pack_drift

**Expected:** grün, ohne `LEAN_MD_BLESS=1`. Das ist das Akzeptanzkriterium: SHA-256 bleibt
SHA-256, also muss `content/skills.sha256` byte-identisch bleiben. Ein roter Lauf mit
Hash-Mismatch bedeutet, dass der Fallback die Hex-Kodierung verändert hat (z. B. Groß- statt
Kleinschreibung) — **nicht** mit `LEAN_MD_BLESS=1` blessen, sondern die Kodierung fixen.

    git diff --stat content/skills.sha256

**Expected:** leere Ausgabe — die Datei ist unverändert.

### Verify & Close

@call verify(Cargo.toml tests/pack_drift.rs)
@call gate(Cargo.toml tests/pack_drift.rs)
@call commit("Cargo.toml Cargo.lock tests/pack_drift.rs", "chore(deps): sha2 0.10 -> 0.11")
@call remember_decision("sha2 0.11 in tests/pack_drift.rs: finalize() liefert hybrid_array::Array; Hex-Kodierung bleibt lowercase, sonst driftet content/skills.sha256")

@phase-end
