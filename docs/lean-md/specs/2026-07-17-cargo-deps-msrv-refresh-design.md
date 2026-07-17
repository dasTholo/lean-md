# Cargo-Deps + MSRV Refresh — Design

**Datum:** 2026-07-17
**Status:** approved
**Scope:** `Cargo.toml` (5 Zeilen), `tests/pack_drift.rs` (3 Aufrufe), `Cargo.lock`

## Ziel

Die Dependencies in `Cargo.toml` auf den aktuellen Stand ziehen und die
`rust-version` auf die letzte Stable heben. Versionsangaben bleiben durchgehend
im Format `major.minor` — nie `major.minor.patch`.

## Ist-Stand

Das `major.minor`-Format ist im Manifest bereits überall eingehalten. Echte
Arbeit sind nur drei Zeilen:

| Dep                | Cargo.toml | crates.io  | Aktion              |
|--------------------|------------|------------|---------------------|
| `rushdown`         | `0.18`     | 0.18.0     | —                   |
| `evalexpr`         | `13.1`     | 13.1.0     | —                   |
| `serde_json`       | `1.0`      | 1.0.150    | —                   |
| `regex`            | `1.12`     | **1.13.1** | → `"1.13"`          |
| `chrono`           | `0.4`      | 0.4.45     | —                   |
| `lean-ctx-client`  | `0.1`      | 0.1.0      | —                   |
| `tiktoken-rs` (dev)| `0.12`     | 0.12.0     | —                   |
| `sha2` (dev)       | `0.10`     | **0.11.0** | → `"0.11"` (breaking)|
| `rust-version`     | `1.96`     | Stable 1.97.1 | → `"1.97"`       |

## Änderungen

```toml
regex = "1.13"          # war "1.12"
rust-version = "1.97"   # war "1.96"
sha2 = "0.11"           # war "0.10"  [dev-dependencies]
```

## Sequenz — drei Commits, je ein Gate

Die Bumps sind unabhängig. Getrennte Schritte machen einen roten Lauf ohne
Bisect zuordenbar — das ist bei einem Breaking-Release der ganze Wert.

### 1. `regex 1.12 → 1.13`

Reiner Minor-Bump, keine Codeänderung erwartet.
**Verifikation:** `cargo nextest run` — Erwartung: grün.

### 2. `rust-version 1.96 → 1.97`

**Verifikation:** `rustup run stable cargo check`.

Der lokale Default ist nightly 1.99 (`rust-toolchain.toml` pinnt nightly für den
Cranelift-Backend-Beschleuniger). Ein normaler `cargo check` prüft die
MSRV-Behauptung deshalb **nicht** — nur der explizite Stable-Lauf beweist, dass
1.97 trägt.

Die MSRV ist nirgends dupliziert: `Cargo.toml:5` ist die einzige Quelle, CI
neutralisiert `rust-toolchain.toml` (`rm -f`) und baut auf Stable. Kein
Doku-Sync nötig.

### 3. `sha2 0.10 → 0.11`

Betrifft ausschließlich `tests/pack_drift.rs:57-76` (`render_manifest()`) — drei
Aufrufe: `Sha256::new()` / `update()` / `finalize()`. Sonst nirgends im Crate.

**Erwartetes Risiko:** `finalize()` liefert in 0.11 ein `hybrid_array::Array`
statt `GenericArray`; das `{:x}`-Formatting (Zeile 73) kann dadurch wegfallen.
**Fallback:** einzeiliger Hex-Encoder.

**Verifikation:** `cargo nextest run --test pack_drift`.
**Akzeptanzkriterium:** `content/skills.sha256` bleibt byte-identisch, das
Drift-Gate läuft **ohne** `LEAN_MD_BLESS=1` grün. Das beweist zugleich, dass die
Migration keine Hashwerte verändert hat (SHA-256 bleibt SHA-256, #498
Byte-Stabilität bleibt gewahrt).

## Abschluss

- `cargo clippy --all-targets` — Zero-Warnings-Bar
- `cargo nextest run` — voller Lauf
- `cargo fmt` pro geänderter Datei vor `git add`

## Nicht-Ziele

- **Kein dauerhaftes Format-Gate.** Entschieden: `major.minor` wird per Review
  gehalten, nicht per Test/CI.
- **Kein `cargo update`** der transitiven Deps. `Cargo.lock` bewegt sich nur,
  soweit die drei Bumps es erzwingen.
- **Kein Anfassen von `rust-toolchain.toml`.** Der Nightly-Pin ist ein
  dev-only Accelerator und unabhängig von der MSRV.

## Offener Trade-off (entschieden)

MSRV 1.97 sperrt crates.io-Konsumenten auf 1.96 aus. Bei `0.2.0` und dem eng
definierten Addon-Nutzerkreis akzeptiert.

## Shell-Hinweis

Projektregel: kein `&&`/`||`/`;`-Chaining — jeder Befehl ist eine eigene
Invocation.
