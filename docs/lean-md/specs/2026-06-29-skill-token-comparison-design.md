# Skill-Token-Vergleich — Design-Spec

Datum: 2026-06-29
Status: Design (bereit für Implementierungsplan)
Repo: lean-md (Branch `feat-lmd-v2`)

## 1. Ziel

Ein reproduzierbares Harness, das zwei TDD-Skills vergleicht:

- **Variante A** — `superpowers/test-driven-development` (monolithisches `SKILL.md`,
  vollständig beim Skill-Invoke in den Kontext geladen, + Companion
  `testing-anti-patterns.md`).
- **Variante B** — `lmd-test-driven-development` (winziger Delegations-Stub
  `SKILL.md` + `body.lmd.md`, **phasenweise on-demand** gerendert via
  `lean-md render` / `ctx_md_render`: `red → green → refactor → rationalizations`,
  plus Companion).

Gemessen wird **Token-Verbrauch** in zwei Schichten: ein deterministischer,
byte-stabiler statischer Trace (Schicht A) plus eine validierende
Subagent-Messung mit Druck-Varianten (Schicht B).

Das Framing ist **neutral A/B** — keine RED/GREEN-Baseline-Rollen. Beide
Varianten werden gleichberechtigt gemessen; Δ wird in beide Richtungen
ausgewiesen.

## 2. Kernmetrik — die ehrliche Zahl

Reine Inhalts-Tokens genügen nicht. Variante B tauscht *einen großen Block*
gegen *N kleine Blöcke + N Tool-Call-Roundtrips*. Jeder `ctx_md_render`-Aufruf
trägt Overhead (Tool-Schema-Anteil, Argumente, Wrapper). Das Harness berichtet
darum **zwei Zahlen pro Variante**:

| Metrik | Variante A (superpowers) | Variante B (lmd) |
|---|---|---|
| **Reiner Inhalt** (Tokens des geladenen Markdown) | `SKILL.md` komplett | Stub + gerenderte Phasen |
| **Inkl. Ablauf-Overhead** (Tool-Calls × Roundtrip-Kosten) | 1× Skill-Load | 1× Stub-Load + N× `ctx_md_render` |

Erst die zweite Zeile zeigt, ob das Just-in-time-Rendering sich **netto** lohnt.
Daraus abgeleitet: die **Break-even-Phasenzahl** — ab wie vielen real erreichten
Phasen Variante B teurer wird als der Monolith.

### Falsifizierbare Kernhypothese

Variante B gewinnt **genau dann**, wenn:

1. typische Aufgaben bei RED/GREEN stoppen (`refactor`/`rationalizations` werden
   nie geladen), **und**
2. der kumulierte Tool-Call-Overhead kleiner ist als die eingesparten
   Inhalts-Tokens.

Das Harness macht beide Bedingungen schwarz auf weiß prüfbar.

## 3. Herkunft der Methodik — adaptiert aus dem mdai-Harness

Quelle: `dasTholo/lean-ctx@feat-lmd-v1`, `docs/mdai/` (`red-baseline/`,
`green-verification/`). Das dortige Harness maß **Macro-Library-Adoption** über
drei Achsen, die wir erben — mit gezielten Änderungen:

| mdai (Original) | dieses Harness (adaptiert) |
|---|---|
| LOC als Proxy | **echte Tokens** (tiktoken) + LOC nur als Quervergleich |
| „Library vs. keine Library" | **superpowers-Monolith vs. lmd-Phasen-Rendering** |
| misst Macro-Adoption / Drift | misst **Lade-Kosten + Ablauf-Overhead** (§2) |
| RED/GREEN-Baseline-Rollen | **neutrales A/B** (kein Baseline-Framing) |
| nur Subagent-Läufe (qualitativ) | **geschichtet**: scripted Trace (deterministisch) **+** Subagent-Validierung |

**Übernommen wird:**

- **Druck-Varianten pro Subagent-Lauf** — `cold` (keine Beschränkung), `time`
  (Zeitdruck), `authority` (Tech-Lead-Override). Kontrolliert die
  Verhaltens-Varianz und macht nicht-deterministische Läufe vergleichbar, weil
  jede Variante denselben Bias auf beide Skills anwendet.
- **Strukturierte Report-Artefakte** in Verzeichnissen + `SUMMARY.md` mit
  Vergleichstabellen.

## 4. Architektur

Das Harness ist ein **in-process Rust-Programm** — kein Python, kein
Subprozess. Begründung: lean-ctx zählt Tokens bereits mit `tiktoken-rs` (siehe
§4.1), und lean-md exponiert das Phasen-Rendering als Bibliotheks-API
(`lean_md::skills::render_skill` / `render_companion`, verifiziert in
`src/skills.rs`). Beides läuft damit im selben Prozess → vollständig
deterministisch, byte-stabil (#498), reproduzierbar ohne externe Toolchain.

```
benchmarks/skill-token-comparison/
  main.rs                  # Schicht A: rendert in-process + tokenisiert + schreibt SUMMARY.md
  variant-A-superpowers/   # Schicht B: Subagent-Reports (cold/time/authority)
  variant-B-lmd/           # Schicht B: Subagent-Reports (cold/time/authority)
  SUMMARY.md               # A/B-Vergleichstabellen (deterministisch, keine Timestamps im Body)
```

Verdrahtung als Cargo-Example mit explizitem Pfad (hält Benchmarks unter
`./benchmarks/` statt im default `examples/`):

```toml
# Cargo.toml
[dev-dependencies]
tiktoken-rs = "0.12"          # exakt die lean-ctx-Version (Cargo.toml:142)

[[example]]
name = "skill-token-comparison"
path = "benchmarks/skill-token-comparison/main.rs"
```

Lauf: `cargo run --example skill-token-comparison` → schreibt `SUMMARY.md`.
Begründung Ablage unter `./benchmarks/` (Repo-Root), nicht `docs/`: Benchmarks
sind ausführbare Artefakte (Code + reproduzierbare Läufe), kein Fließtext.

### 4.1 Token-Zählung — `tiktoken-rs` als dev-dependency

lean-ctx zählt mit `tiktoken-rs = "0.12"` (`rust/src/core/tokens.rs`). Wir
spiegeln dessen Ansatz minimal im Harness (kein moka/blake3-Cache — für einen
One-Shot-Lauf YAGNI):

```rust
let bpe = tiktoken_rs::cl100k_base()?;          // Claude-Familie
let tokens = bpe.encode_with_special_tokens(text).len();
```

**Zwei Familien, zwei Spalten im Report:**

- **`cl100k_base` — primäre Wahrheit.** Diese Skills werden von **Claude**
  konsumiert; `core/tokens.rs` dokumentiert `Cl100k` als „within ~3% of Claude's
  actual tokenizer" und `detect_tokenizer("opus"|"sonnet"|"claude") -> Cl100k`.
- **`o200k_base` — Parität.** lean-ctx's eigenes Savings-Ledger zählt mit
  `O200kBase` (`COUNTING_FAMILY`). Als zweite Spalte ausgewiesen, damit unsere
  Zahlen mit dem lean-ctx-Ledger quervergleichbar sind.

Bewusste Einschränkung: `cl100k_base` ~ Claude-Tokenizer (~3% Abweichung), nicht
exakt. Der **A/B-Trend** bleibt unberührt; im `SUMMARY.md` explizit vermerkt.
Die frühere `tokens saved`-Heuristik des lean-ctx-Servers wird **nicht** als
Wahrheit benutzt.

**Kein `#[cfg]`-Gate nötig.** Der gesamte Tokenizer-Code lebt im
`[[example]]`-Target (`benchmarks/skill-token-comparison/main.rs`).
Cargo kompiliert Examples separat (nur bei `cargo build --examples` /
`cargo run --example` / `cargo test`) und bundlet sie **nie** in den
Lib-/Bin-`--release`-Build; `dev-dependencies` sind dort automatisch sichtbar.
Ein `#[cfg(...)]`-Gate bräuchte man erst, wenn der Tokenizer in `src/` referenziert
würde — dort sind dev-deps nur unter `#[cfg(test)]` verfügbar (normaler Lib-Code
sieht sie nicht). **Der Implementierungsplan darf daher kein `cfg`-Gate einbauen**
und den Helper nicht nach `src/` ziehen (YAGNI). So bleibt `lean_md`
tokenizer-frei.

### 4.2 Schicht A — Scripted Trace (deterministisch, in-process)

`main.rs`:

- **Variante A**: liest `SKILL.md` (superpowers) komplett + `testing-anti-patterns.md`
  vom Pfad und tokenisiert (Companion nur gezählt, wenn ein realer Lauf ihn lädt
  -> §4.3). Der superpowers-Skill-Pfad ist eine benannte Konstante (Plugin-Cache;
  im Report offengelegt).
- **Variante B**: rendert in-process via
  `render_skill("lmd-test-driven-development", Some(phase), Consumer::Ai, …)` je
  Phase (`red/green/refactor/rationalizations`) + Stub-`SKILL.md` +
  `render_companion(…, "testing-anti-patterns", …)`; tokenisiert jeden Output.
- Pro Artefakt: Token-Count (beide Familien). Aggregiert zur
  **2-Zeilen-Kernmetrik** (§2) + **Break-even-Phasenzahl**.
- **Tool-Call-Overhead-Modell**: konstante Tokenzahl pro `ctx_md_render`-Roundtrip
  (Tool-Schema-Anteil + Argumente + Wrapper). Als benannte Konstante im Code
  geführt und im `SUMMARY.md` offengelegt — nachvollziehbar/justierbar.
- Determinismus (#498): Output ist reine Funktion von (Datei-Inhalt, Phase,
  CRP-Modus, Tokenizer-Familie). Keine Timestamps/Zähler im Report-Body.

### 4.3 Schicht B — Subagent-Validierung (mdai-Stil)

- Dieselbe **Mini-TDD-Aufgabe** (eine kleine, klar umrissene Funktion + Bugfix)
  wird je Variante gelöst — einmal pro Druck-Variante `cold`/`time`/`authority`.
- Jeder Subagent-Report hält **verbatim** fest, welche Skill-Artefakte real
  geladen wurden (Variante B: welche Phasen tatsächlich gerendert wurden) und
  wie viele Tool-Calls anfielen.
- Das Harness (`main.rs`, dieselbe `tiktoken-rs`-Zählung wie Schicht A) zählt die
  geladenen Artefakte nach → realer kumulierter Verbrauch.
- Zweck: bestätigt oder falsifiziert die Schicht-A-Hypothese (§2.1) — insbes.
  ob reale Agenten bei RED/GREEN stoppen oder doch alle Phasen abrufen.
- Reports landen unter `variant-A-superpowers/` bzw. `variant-B-lmd/`, benannt
  nach Druck-Variante.

### 4.4 Report: `SUMMARY.md`

A/B-Vergleichstabellen:

- Token pro Artefakt (beide Varianten).
- Summen + **Δ absolut / Δ %** (in beide Richtungen ausgewiesen).
- **Break-even-Phasenzahl**.
- Schicht-B-Beobachtungen: welche Phasen ein realer Agent je Druck-Variante
  tatsächlich abruft; Abweichung Schicht A ↔ Schicht B.
- Offengelegte Annahmen: Tokenizer-Hinweis (§4.1), Tool-Call-Overhead-Konstante
  (§4.2).

## 5. Abgrenzung (YAGNI)

- **Keine** Latenz-/Wall-Clock-Messung (nur Tokens — so vom User bestätigt).
- **Keine** Multi-Modell-Tokenizer-Matrix über cl100k/o200k hinaus; diese zwei
  Familien (§4.1) genügen für Trend + lean-ctx-Parität.
- **Kein** moka/blake3-Token-Cache (lean-ctx-Optimierung) — für einen
  One-Shot-Benchmark unnötig.
- **`tiktoken-rs` nur als `dev-dependency`** — die Render-Bibliothek `lean_md`
  bleibt frei vom Tokenizer (kein Eintrag in `[dependencies]`).
- **Kein** RED/GREEN-Rollen-Framing (neutrales A/B).

## 6. Erfolgskriterien

- `cargo run --example skill-token-comparison` produziert byte-stabilen
  `SUMMARY.md`-Block bei wiederholtem Lauf (gleiche Eingabe → gleiche Bytes).
- `SUMMARY.md` beantwortet eindeutig: (a) reiner Inhalt A vs. B, (b) inkl.
  Overhead A vs. B, (c) Break-even-Phasenzahl, (d) ob Schicht-B-Läufe die
  Hypothese stützen.
- Methodik + Annahmen (Tokenizer, Overhead-Konstante, Druck-Varianten) sind im
  Report transparent dokumentiert.
