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

```
benchmarks/skill-token-comparison/
  tokenize.py              # tiktoken cl100k_base — deterministische Token-Wahrheit
  collect_static.py        # Schicht A: ruft `lean-md render` je Phase, zählt Artefakte
  variant-A-superpowers/   # Schicht B: Subagent-Reports (cold/time/authority)
  variant-B-lmd/           # Schicht B: Subagent-Reports (cold/time/authority)
  SUMMARY.md               # A/B-Vergleichstabellen (deterministisch, keine Timestamps im Body)
```

Begründung Ablage unter `./benchmarks/` (Repo-Root), nicht `docs/`: Benchmarks
sind ausführbare Artefakte (Skripte + reproduzierbare Läufe), kein Fließtext.

### 4.1 Komponente: `tokenize.py`

- Tokenizer: tiktoken `cl100k_base`. **Primäre Wahrheit.**
- Begründung: lean-md hat **keinen eigenen Tokenizer** (verifiziert: 0 Treffer
  für `tiktoken`/`count_tokens` in `src/`). Die `tokens saved`-Metrik stammt vom
  **lean-ctx-Server** (separate Komponente) und ist Heuristik — daher nur
  optionaler Quervergleich, nicht die Wahrheit.
- Schnittstelle: zählt Tokens einer Datei oder eines Strings; deterministisch,
  offline, kein Netzwerk, kein API-Key.
- Bewusste Einschränkung: tiktoken ≠ exakter Claude-Tokenizer. Absolute Zahlen
  weichen leicht ab; der **A/B-Trend** bleibt valide. Wird im `SUMMARY.md`
  explizit vermerkt.

### 4.2 Schicht A — Scripted Trace (deterministisch)

`collect_static.py`:

- **Variante A**: tokenisiert `SKILL.md` komplett + `testing-anti-patterns.md`
  (Companion nur gezählt, wenn der Skill-Text ihn referenziert / ein realer Lauf
  ihn lädt → siehe §4.3).
- **Variante B**: tokenisiert Stub-`SKILL.md` + Output von
  `lean-md render --consumer=ai` je Phase (`red/green/refactor/rationalizations`)
  + Companion `testing-anti-patterns.lmd.md`.
- Pro Artefakt: Token-Count. Aggregiert zur **2-Zeilen-Kernmetrik** (§2) +
  **Break-even-Phasenzahl**.
- **Tool-Call-Overhead-Modell**: konstante Tokenzahl pro `ctx_md_render`-Roundtrip
  (Tool-Schema-Anteil + Argumente + Wrapper). Der Wert wird als benannte
  Konstante im Skript geführt und im `SUMMARY.md` offengelegt, damit das Modell
  nachvollziehbar/justierbar ist.
- Determinismus (#498): Output ist reine Funktion von (Datei-Inhalt, Phase,
  CRP-Modus). Keine Timestamps/Zähler im Report-Body.

### 4.3 Schicht B — Subagent-Validierung (mdai-Stil)

- Dieselbe **Mini-TDD-Aufgabe** (eine kleine, klar umrissene Funktion + Bugfix)
  wird je Variante gelöst — einmal pro Druck-Variante `cold`/`time`/`authority`.
- Jeder Subagent-Report hält **verbatim** fest, welche Skill-Artefakte real
  geladen wurden (Variante B: welche Phasen tatsächlich gerendert wurden) und
  wie viele Tool-Calls anfielen.
- `tokenize.py` zählt die geladenen Artefakte nach → realer kumulierter
  Verbrauch.
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
- **Keine** Multi-Modell-Tokenizer-Matrix; ein Tokenizer (cl100k) genügt für den
  Trend.
- **Kein** eigener Rust-Tokenizer in lean-md (würde schwergewichtige Dependency
  `tiktoken-rs` ziehen; Python-Skript ist pragmatischer und reicht für ein
  Benchmark-Harness).
- **Kein** RED/GREEN-Rollen-Framing (neutrales A/B).

## 6. Erfolgskriterien

- `collect_static.py` produziert byte-stabilen `SUMMARY.md`-Block bei
  wiederholtem Lauf (gleiche Eingabe → gleiche Bytes).
- `SUMMARY.md` beantwortet eindeutig: (a) reiner Inhalt A vs. B, (b) inkl.
  Overhead A vs. B, (c) Break-even-Phasenzahl, (d) ob Schicht-B-Läufe die
  Hypothese stützen.
- Methodik + Annahmen (Tokenizer, Overhead-Konstante, Druck-Varianten) sind im
  Report transparent dokumentiert.
