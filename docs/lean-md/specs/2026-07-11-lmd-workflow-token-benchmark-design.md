# Design: `workflow-token-comparison` — 3-Wege-Workflow-Benchmark

Datum: 2026-07-11 · Status: Design freigegeben · Autor: Claude (Opus) via lmd-brainstorm
Nachfolge-Skill: lmd-writing-plans

---

## 1. Ziel

Ein deterministischer, byte-stabiler (#498) Benchmark, der die Skill-Kette
**brainstorm → writing-plans → subagent-driven-development** über drei Systeme
gegenüberstellt:

- **superpowers** — monolithische `SKILL.md`, keine Render-Engine.
- **mdai (markdownai)** — Vorläufer von lean-md; Node-Render-Engine, aber nur
  *ein* Skill (`mdai-brainstorm`) je gebaut.
- **lean-md** — phasen-gerenderte Skills + Render-Engine (in-process).

Zwei Metrik-Achsen:

- **Achse 1 — Skill-Instruktions-Overhead** pro Stufe: wie viele Anweisungstokens
  injiziert ein Skill, wenn ein Agent ihn nutzt.
- **Achse 2 — Plan-Artefakt-Größe + Phase-isolierter Dispatch**: wie groß ein Plan
  in Input-Tokens ist (Source / Full-Render / pro isolierter Phase) und was der
  Subagent-Dispatch kostet.

Der Benchmark erweitert die zwei bestehenden Präzedenzfälle:
`benchmarks/skill-token-comparison/` (Achse 1, ein Skill) und
`lean-ctx:/mdai-benchmark.md` (Achse 2, ein Plan).

## 2. Nicht-Ziele (YAGNI)

- **Keine** Live-Agenten-Messung. Token werden deterministisch gerendert +
  tokenisiert, Workflow-Verbrauch über ein offengelegtes Kostenmodell hochgerechnet.
- **Kein** Bau/Nutzung der im mdai-Worktree mitliegenden **lean-ctx** — sie ist
  viele Versionen zurück; ein Vergleich darüber misst das Falsche und wäre unfair.
  Gemessen wird ausschließlich der **markdownai**-Render-Layer.
- **Keine** Behauptung eines mdai-Head-to-Head über alle Stufen: mdai hat nur einen
  brainstorm-Skill (siehe §3). Fehlende Stufen werden als N/A ausgewiesen.
- **Keine** hartkodierten Preisbehauptungen: Preise und Overhead-Annahmen sind
  offengelegte, justierbare Konstanten.

## 3. Kontrahenten-Fähigkeitsmatrix (die Asymmetrie)

| Fähigkeit | superpowers | mdai (markdownai) | lean-md |
|---|---|---|---|
| brainstorm-Skill | Monolith `.md` | `mdai-brainstorm` (.mdai-gerendert) | phased |
| writing-plans-Skill | Monolith `.md` | **N/A — nie gebaut** | phased |
| subagent-Skill | Monolith `.md` | **N/A — nie gebaut** | phased |
| Render-Engine / Phase-Isolation | **keine** → Dispatch = Vollplan | ✓ Engine | ✓ Engine |

Belegt: `mdai-bench`-Worktree trägt genau einen Skill (`mdai/skills/mdai-brainstorm/`);
superpowers 6.0.3 und lean-md tragen alle drei.

**Konsequenz:** mdai nimmt auf **Achse 1 nur an der brainstorm-Stufe** teil, auf
**Achse 2 voll** (die markdownai-Engine rendert phase-isoliert — der strukturelle
Hebel, den mdai bewies und den lean-md auf alle drei Skills generalisierte).

## 4. Achse 1 — Skill-Instruktions-Overhead

Für jedes Paar (Stufe × Kontrahent) werden die tatsächlich injizierten Artefakte
tokenisiert. Tokenizer: `cl100k_base` (primär), `o200k_base` (Parität) — identisch
zu `skill-token-comparison`.

- **superpowers**: `SKILL.md`-Monolith **+ Companions up front** (voll geladen).
- **lean-md**: Stub + on-demand gerenderte Phasen + on-demand Companions →
  **kumulative Break-even-Tabelle** (Stub + k Phasen), Muster aus `skill-token-comparison`.
- **mdai**: nur brainstorm; `mdai-brainstorm/body.mdai.md` per markdownai-Engine
  gerendert. writing-plans / subagent = **N/A** (mit Begründung im Output).

Zusatzausgabe: **brainstorm-3-Wege-Direktvergleich** (die einzige Stufe, in der
alle drei existieren).

## 5. Achse 2 — Plan-Artefakt & Dispatch

**Zwei Fixtures, zwei Genres** (Refactoring + Audit — analog v3 S3a vs. v4 Part-B im
mdai-benchmark), je **3-fach portiert mit identischer Substanz**:
`plan.md` (superpowers-Prosa-Monolith), `plan.mdai.md`, `plan.lmd.md`.
Quelle: reale Pläne aus `docs/lean-md/plans/` (konkrete Auswahl in der Plan-Phase).

Pro Fixture gemessen (jeweils cl100k):

| Messpunkt | superpowers | mdai | lean-md | Bedeutung |
|---|---|---|---|---|
| Source-Tokens | plain | Direktiven | Direktiven | Schreibarbeit |
| Full-Render | = Source | gerendert | gerendert | Orchestrator liest |
| Pro-Phase isoliert | = Vollplan | `read_file(phase=)` | `render --phase` | Subagent bekommt |
| Dispatch (N Subagents) | N × Vollplan | N × Phase + Overhead | N × Phase + Overhead | Kostenmodell |

**Fairness-Protokoll:** die drei Ports MÜSSEN inhaltsgleich sein. Der Harness führt
einen **Sanity-Check** auf die gerenderten Outputs (Token-Deltas innerhalb Toleranz),
damit kein Format durch Weglassen von Inhalt „gewinnt". Autoren-Protokoll +
Toleranzwert werden in `README.md` dokumentiert.

## 6. Kostenmodell

Wie `mdai-benchmark.md` §„Subagent-Dispatch Cost-Model":
Input-Tokens × Subagent-Anzahl × Preis/1M, Baseline = Vollplan-Dispatch.

- Preise (`PRICE_SONNET_PER_M`, `PRICE_OPUS_PER_M`) und
  `TOOL_CALL_OVERHEAD_TOKENS` / `HARD_RULES_OVERHEAD_TOKENS` sind **named constants**,
  im generierten `SUMMARY.md` offengelegt und justierbar (Disziplin wie
  `TOOL_CALL_OVERHEAD_TOKENS` im bestehenden Benchmark).
- Keine Preisannahme wird als Faktum behauptet; Zahlen sind Modell-Parameter.

## 7. mdai-Adapter (`src/mdai.rs`)

Rust-Modul, das **ausschließlich die markdownai-Render-Engine** im Worktree
`/home/tholo/Scripts/lean-ctx-mdai-bench` als Subprozess aufruft (Pfad
env-konfigurierbar, z.B. `MDAI_WORKTREE`). Rendert Full + Pro-Phase; die
stdout-Bytes werden mit **demselben tiktoken-rs** tokenisiert → garantierte
Tokenizer-Parität über alle drei Systeme.

**Die mitgelieferte lean-ctx wird nicht gebaut und nicht aufgerufen** (§2).

**Preflight** prüft nur markdownai-Baubarkeit/-Lauffähigkeit. Schlägt er fehl,
bricht der Bench **nicht hart ab**, sondern schaltet in den **recorded-Modus** (§9).

> **Offener Impl-Punkt (in writing-plans zu klären):** markdownai-Phase-Isolation
> lief historisch nur über den **MCP-Server** (`read_file(phase=)`), nicht per
> CLI-Flag. Der Plan prüft, ob der `mdai-bench`-Branch ein CLI-`--phase` besitzt;
> falls nicht, treibt der Adapter den MCP-Server per stdio (inkl.
> Compliance-Patch `respondTool()` aus `mdai-benchmark.md`), oder es greift §9.

## 8. Determinismus (#498)

- tiktoken-rs ist deterministisch; markdownai-Render ist rein → byte-stabil.
- Keine Timestamps/Counter im `SUMMARY.md`-Body.
- Preise/Overhead/Toleranz als offengelegte Konstanten.
- Recorded-Fallback-Zahlen (§9) sind Konstanten → ebenfalls byte-stabil.
- Der mdai-Status wird pro Zelle transparent markiert: `live` | `recorded` | `N/A`.

## 9. Graceful Degradation (mdai)

Live-markdownai-Render ist das Ziel, mit dokumentiertem Fallback. Wird der
Vergleich **zu komplex oder nicht lauffähig** (Build bricht, Phase-Isolation nur
über gepatchten MCP-Server nicht verfügbar), dann:

1. mdai-Zellen aus dem bestehenden **`mdai-benchmark.md` zitieren** (v3 S3a / v4
   Part-B), im Output als **`recorded`** markiert statt `live`.
2. explizit festhalten: **markdownai diente als Inspiration/Vorläufer für lean-md** —
   kein Head-to-Head-Anspruch über die Engine, sondern Lineage.

Der Umschaltpunkt (live ↔ recorded) ist ein Preflight-Ergebnis, kein manueller
Schalter; das SUMMARY dokumentiert, welcher Modus aktiv war.

## 10. Deliverables / Dateilayout

```
Cargo.toml                       (root → wird [workspace], fügt member hinzu)
benchmarks/workflow-token-comparison/
  Cargo.toml                     (crate: workflow-token-comparison; deps: lean_md, tiktoken-rs)
  src/main.rs                    (collect → compute → write SUMMARY)
  src/harness.rs                 (Family, Artifact, token_count — Form wie skill-token-comparison)
  src/axis1.rs                   (Overhead je Stufe/Kontrahent)
  src/axis2.rs                   (Plan source/render/phase + Kostenmodell)
  src/mdai.rs                    (markdownai-Node-Subprozess-Adapter + Preflight + recorded-Fallback)
  fixtures/refactoring/{plan.md, plan.mdai.md, plan.lmd.md}
  fixtures/audit/{plan.md, plan.mdai.md, plan.lmd.md}
  SUMMARY.md                     (generiert, byte-stabil)
  README.md                      (Methodik, markdownai-Build-Prereq, Fairness-Protokoll, recorded-Modus)
  tests/stability.rs             (byte-stabil + N/A/recorded-Invarianten, via nextest)
```

- Aufruf: `cargo run -p workflow-token-comparison`
- Test: `cargo nextest run -p workflow-token-comparison`
- **Nicht** als `[[example]]` (bewusste Abgrenzung zu `skill-token-comparison`):
  eigenständige Workspace-Member-Crate, damit `tiktoken-rs` + Subprozess-Logik
  vollständig aus der publizierten `lean_md`-Crate herausbleiben.

## 11. Prerequisites & Risiken

| Punkt | Status / Mitigation |
|---|---|
| markdownai-Build im Worktree (npm + tsc) | Preflight; scheitert → recorded-Modus (§9) |
| lean-ctx im mdai-Worktree | **nicht bauen/nutzen** (§2) |
| markdownai-Phase-Isolation CLI vs. MCP | offener Impl-Punkt (§7), in writing-plans klären |
| Fixture-Portierung (2 Pläne × 2 Fremd-Formate) | manuelle Autorenarbeit; `.lmd.md` existiert real |
| Root wird [workspace] | einmalige Cargo-Umstellung, single-crate bleibt baubar |

## 12. Akzeptanzkriterien

1. `cargo run -p workflow-token-comparison` erzeugt ein byte-stabiles `SUMMARY.md`
   mit beiden Achsen und pro mdai-Zelle sichtbarem `live`/`recorded`/`N/A`-Status.
2. `cargo nextest run -p workflow-token-comparison` grün: Byte-Stabilität +
   N/A-Invarianten (mdai writing-plans/subagent = N/A) asserted.
3. Tokenizer-Parität: alle drei Systeme über dieselbe tiktoken-rs-Instanz gemessen.
4. Kein Aufruf der mdai-Worktree-lean-ctx; markdownai-Layer isoliert gemessen.
5. Fairness-Sanity-Check auf gerenderte Fixture-Outputs vorhanden und dokumentiert.
6. `lean_md`-Crate bleibt frei von `tiktoken-rs` und Subprozess-Logik.
