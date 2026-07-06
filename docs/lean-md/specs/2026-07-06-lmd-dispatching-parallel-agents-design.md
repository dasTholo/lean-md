# Design-Spec: lmd-dispatching-parallel-agents

**Datum:** 2026-07-06
**Status:** genehmigt (Brainstorm), bereit für Planung
**Vorlage:** `superpowers/6.1.1/skills/dispatching-parallel-agents`

## Ziel

Den superpowers-Skill `dispatching-parallel-agents` als nativen lmd-Skill
`lmd-dispatching-parallel-agents` portieren — **vollständiger Ersatz ohne
Funktionsverlust** — UND dieselbe Fähigkeit als **opt-in-Modus** in
`lmd-subagent-driven-development` (SDD) verankern. Beide Konsumenten teilen sich
**ein** wiederverwendbares Fragment als Single Source (`@include parallel-dispatch`).

Der Port nutzt lean-md-Funktionen (`@phase`-Isolation, `@include`-Fragment,
`@dispatch`, `@call`, `@query`) und bewahrt den lean-ctx Memory/Coordination-
Kontrakt (`ctx_session` / `ctx_knowledge` / `ctx_agent`) identisch zum bestehenden
sequenziellen SDD-Pfad.

## Nicht-Ziele (YAGNI)

- **Kein Worktree-Zwang** — Unabhängigkeit paralleler Agents = **disjunkte Dateien**
  ist Vorbedingung (Original: „Don't use when shared state / agents edit same
  files"). Worktree-Isolation wird bewusst nicht eingebaut.
- **Keine neue Koordinations-Infra** — `ctx_agent` (post/handoff/sync) + der
  session-weit geteilte MCP-Cache genügen; kein `ctx_share`, keine SDD-bash-Skripte.
- **Kein Aufweichen des Zwei-Verdict-Reviews** — der parallele Pfad bewahrt das
  Per-Task-Review; nur die Ausführung fächert auf.

## Architektur

**Single Source + zwei Konsumenten** (Brainstorm-Entscheidung 1):

```
content/core/_fragments/parallel-dispatch.lmd.md   ← Single Source (Fragment)
  ├─ skills/lmd-dispatching-parallel-agents/body.lmd.md   @include parallel-dispatch
  └─ skills/lmd-subagent-driven-development/body.lmd.md    @include parallel-dispatch (gated phase)
```

Das Fragment ist ein flacher globaler Name (`parallel-dispatch`), built-in via
`include_str!` in `src/fragments.rs` (byte-stabil #498), mit on-disk-Consistency-
Gate — Präzedenz: `test-first-core`, `skill-authoring-core`, `brainstorm-gate`.

### Single Source — Fragment `parallel-dispatch`

Inhalt (verlustfrei zum Original, aber terse):

- **When-to-use-Gate** (Entscheidungsbaum): mehrere Fehler? → unabhängig? → shared
  state? → parallel vs. sequenziell vs. single-agent.
- **Kernprinzip:** „one `@dispatch` per independent problem domain".
- **Fan-out-Regel:** **mehrere `@dispatch` in EINER Antwort = parallel**; eine pro
  Antwort = sequenziell.
- **Prompt-Struktur:** focused / self-contained / output-spec (jeder Agent bekommt
  scope + goal + constraints + expected output).
- **Common Mistakes** (terse Inline-Referenz): zu breit / kein Kontext / keine
  Constraints / vage Output-Spec.
- **Verification:** je Summary lesen → Konflikt-Scan (gleiche Datei editiert?) →
  volle Suite → Spot-Check.
- **Memory/Coordination-Pflichtblock** (siehe unten) — so dass **beide** Konsumenten
  (auch der Standalone-Skill) die `ctx_*`-Disziplin erben.

### Konsument A — Standalone-Skill `lmd-dispatching-parallel-agents`

Rein interaktiver Inline-Skill (main agent), render-on-demand, terminaler Zustand
nach Integration; kein Folge-Skill.

| Phase | Zweck | lean-md-Funktionen |
|---|---|---|
| `pre-context` | Announce + Ambient-Baseline | `@include hard-rules` |
| `assess` | When-to-use-Gate; unabhängige Domänen gruppieren | `@include parallel-dispatch` |
| `dispatch` | Fan-out: mehrere `@dispatch` in **einer** Antwort | `@dispatch` (Contract auto-prepended) |
| `integrate` | Konflikt-Scan → Integration → volle Suite | `@call gate(<paths>)`, `@query` |

`assess` und `dispatch` teilen sich den Fragment-Inhalt via `@include`; die
Phasen-Prosa bleibt minimal (Token-Hebel).

### Konsument B — SDD-Integration (neue gated Phase)

Neue Phase **`dispatch-mode`** zwischen `preflight` und `dispatch`
(Brainstorm-Entscheidung 2 — eigene Gate-Phase):

- **`dispatch-mode`** kapselt ausschließlich Abfrage + Verzweigung:
  - Erkennt unabhängige Task-Gruppen aus der `preflight`-Enumeration (disjunkte
    Datei-/Subsystem-Mengen, keine sequenzielle Abhängigkeit).
  - **Fragt den User:** „N unabhängige Tasks parallel dispatchen?" (nur wenn ≥2
    unabhängige Gruppen existieren; sonst direkt sequenziell, keine Frage).
  - **true** → `next: parallel-dispatch`.
  - **false** (oder keine unabhängigen Gruppen) → `next: dispatch` (bestehender Pfad,
    unverändert).
- **`preflight`** endet künftig mit `next: dispatch-mode` statt `next: dispatch`.
- **Neue Phase `parallel-dispatch`** (SDD): `@include parallel-dispatch`; fächert
  mehrere implementer-Subagents in **einer** Antwort auf, dann **pro Rückkehr**
  BASE..HEAD-Review wie `review`, dann Konflikt-Scan + Integration, dann zurück in
  den normalen Fluss (`final-review` unverändert).

### Review-Fidelity (Brainstorm-Entscheidung 3 — Fan-out, Review je Rückkehr)

Paralleler Pfad bewahrt das Zwei-Verdict-Prinzip pro Task:

1. **Vor** dem Fan-out: `BASE_i = @query "git rev-parse HEAD"` **je Agent** merken
   (Fidelity-kritisch; nie `HEAD~1`). Quell-Dateien warm via `ctx_multi_read`.
2. Fan-out: mehrere `@dispatch skill=... companion="implementer" ...` in **einer**
   Antwort.
3. **Je zurückkehrendem Agent:** BASE_i..HEAD-Review (Spec-Compliance +
   Code-Quality) — Reviewer holt Diff selbst (`@read mode=diff`), traut dem Report
   nicht.
4. **Konflikt-Scan:** editierte mehr als ein Agent dieselbe Datei? → gemeinsam
   auflösen, bevor integriert wird.
5. Integration → volle Suite (`@call gate()`).
6. `final-review` (whole-branch) bleibt unverändert.

## Memory / Coordination (verbindlich — deckt den User-Punkt ab)

Der parallele Pfad nutzt **dieselben** lean-ctx-Tools wie `dispatch`/`review`, nur
fan-out-fähig — kein Aufweichen:

- **Fortschritt** → `ctx_session action=task "Task N [x%]"` **pro zurückkehrendem
  Agent** (nicht gesammelt am Ende).
- **Durable Facts** → `ctx_knowledge action=remember` (Entscheidungen/Gotchas je
  Agent) + `@call task_return(...)` → destilliert in Parent-Knowledge (identisch zur
  `review`-Phase).
- **Baton/Status** → jeder Subagent `ctx_agent action=post category=status` +
  `action=handoff to_agent=<controller>`; der Controller pollt **nicht** manuell,
  sondern `ctx_agent action=sync` über die Fan-out-Gruppe.
- **Warm-Read** vor Dispatch → `ctx_multi_read paths=[…]` (geteilter MCP-Cache; kein
  `ctx_share`, kein `fresh`).
- **Dispatch-Contract** wird bei jedem `@dispatch` auto-vorangestellt (trägt die
  `ctx_*`-Tool-Disziplin in jeden Subagent).

Dieser Block lebt im Fragment, damit **auch der Standalone-Skill** ihn erbt — nicht
nur der SDD-Konsument.

## Datenfluss / Kontrollfluss

**Standalone-Skill:**
1. `pre-context`: Baseline + Announce.
2. `assess`: When-to-use-Gate → unabhängige Domänen gruppieren (oder abbrechen →
   sequenziell/single-agent, wenn nicht unabhängig).
3. `dispatch`: Fan-out mehrerer `@dispatch` in einer Antwort.
4. `integrate`: Konflikt-Scan → Integration → volle Suite. Terminal (`ctx_session
   action=status`).

**SDD-Integration:**
1. `preflight` (unverändert außer `next:`) → `dispatch-mode`.
2. `dispatch-mode`: unabhängige Gruppen? → User-Abfrage → `parallel-dispatch` |
   `dispatch`.
3. `parallel-dispatch`: BASE_i je Agent → Fan-out → Per-Rückkehr-Review →
   Konflikt-Scan → Integration → zurück zum normalen Fluss.
4. `final-review` / `handoff`: unverändert.

## Rewiring (bestehende SDD-Verweise)

- `content/skills/lmd-subagent-driven-development/body.lmd.md`: `preflight`-`next:`
  von `dispatch` auf `dispatch-mode` umbiegen; `dispatch-mode` + `parallel-dispatch`
  Phasen einfügen; `dispatch`-Phase selbst bleibt sequenziell und unverändert.
- Der Standalone-Skill ist zusätzlich in der Skill-Description als „use when facing
  2+ independent tasks" auffindbar (Discovery-Parität zum Original).

## Registrierung (analog finishing/executing-plans)

1. **Fragment:** `content/core/_fragments/parallel-dispatch.lmd.md` (Seed) +
   `src/fragments.rs` (`include_str!` in die Registry + Consistency-Gate-Test
   built-in == on-disk).
2. **Skill-Dateien:** `content/skills/lmd-dispatching-parallel-agents/{SKILL.md,
   body.lmd.md}` (Delegations-Stub + Body) + Root-`.claude/skills/`-Stub für
   Claude-Code-Discovery (prüfen, ob install-generiert — analog bestehender Skills).
3. `src/skills.rs` — `include_str!` Body → `SKILLS`-Registry + Render-Test (alle
   Phasen non-empty; `pre-context` trägt hard-rules; `assess`/`dispatch` tragen das
   `parallel-dispatch`-Fragment).
4. `src/skill_install.rs` — `include_str!` SKILL.md → `INSTALLABLE_SKILLS` +
   Install-Test.
5. `src/availability.rs` — `COVERAGE`-Rows (Phase→Tool-Mapping) für die 4
   Standalone-Phasen + die 2 neuen SDD-Phasen + Coverage-Test.
6. SDD-Body-Edit (Rewiring, s. o.).
7. `cargo fmt` je geänderte Datei vor `git add`; `cargo nextest run` grün, zero
   clippy warnings.

## Fidelity-Matrix (Original → Port, jede Einheit abgedeckt)

| Original | Port |
|---|---|
| Overview + Core principle | Fragment: Kernprinzip-Zeile |
| „When to Use" Entscheidungsbaum | Fragment: When-to-use-Gate (`assess` / `dispatch-mode`) |
| Pattern 1 Identify Independent Domains | `assess` / `dispatch-mode` Gruppierung |
| Pattern 2 Create Focused Agent Tasks | Fragment: Prompt-Struktur |
| Pattern 3 Dispatch in Parallel (mehrere in einer Antwort) | `dispatch` / `parallel-dispatch` Fan-out-Regel |
| Pattern 4 Review and Integrate | `integrate` / Per-Rückkehr-Review + Konflikt-Scan |
| Agent Prompt Structure | Fragment: focused/self-contained/output-spec |
| Common Mistakes | Fragment: terse Inline-Referenz |
| When NOT to Use | Fragment: When-to-use-Gate (Nein-Zweige) |
| Real Example / Real-World Impact | Fragment: terse Inline-Referenzzeile (kein Prosa-Block) |
| Key Benefits | Fragment: terse Inline-Referenzzeile |
| Verification | `integrate` / Per-Rückkehr-Review + volle Suite |

## Determinismus (#498)

Fragment + beide Bodies als `include_str!`-Seeds byte-identisch (built-in ==
on-disk). Keine Timestamps/Counter im Render-Output. Fragment-Consistency-Gate muss
grün bleiben.

## Bewusste Planungs-Deferrals (keine Design-Löcher)

- Genaue `COVERAGE`-Row-Tupel (Phase→Tool) beim Planen festlegen — orientiert an den
  tatsächlich genutzten `ctx_*`-Tools je Phase.
- Exakter Wortlaut der `dispatch-mode`-User-Abfrage (Schwelle „≥2 unabhängige
  Gruppen", Formulierung) beim Planen fixieren.
- Prüfen, ob ein Root-`.claude/skills/`-Stub install-generiert wird (analog
  bestehender Skills) oder manuell anzulegen ist.
- Ob `@call task_return(...)` im parallelen Pfad 1:1 aus `review` übernommen wird
  oder eine fan-out-fähige Variante braucht — beim Planen gegen die Makro-Definition
  prüfen.
