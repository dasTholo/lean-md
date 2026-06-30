# `lmd-writing-skills` — Testing-Companion-Ref-Closure + Dangling-Ref-Guard — Design

> **Status:** Design abgeschlossen, bereit für Implementation-Plan.
> **Auslöser:** `ctx_md_render(skill="lmd-writing-skills", companion="testing-skills-with-subagents")`
> löst nicht auf — `COMPANION_NOT_FOUND`.
> **Vorgänger-Kette:** `2026-06-29-lmd-writing-skills-port-design.md` (Monolith-Companion)
> → `2026-06-29-companion-split-dispatch-design.md` (Split in `testing/*`, alten Namen entfernt).
> **Branch:** `feat-lmd-v2`

## 1. Kontext & Ursache

Der Writing-Skills-Port (Spec) lieferte **einen** Companion `testing-skills-with-subagents`;
die Prosa-Verweise im Body und in `bulletproofing` wurden passend dazu geschrieben. Der
**Companion-Split** (Folge-Spec) zerlegte diesen Monolithen bewusst in drei granulare
Companions und **entfernte den alten Seed/Registry-Namen**:

- `testing/methodology` — RED→GREEN→REFACTOR-Testmethodik (Pressure-Szenarien, Rationalization-Tables)
- `testing/skill-types` — wie discipline/technique/pattern/reference-Skills getestet werden
- `testing/creation-checklist` — TDD-adaptierte Checkliste vor dem Deploy

Die **Reference-Closure** des Split-Specs schloss nur Querverweise **zwischen** den drei
neuen Dateien (Detail 1, Regel „Reference-Closure"), übersah aber die **eingehenden**
Prosa-Verweise aus `body.lmd.md` und `bulletproofing.lmd.md` auf den gelöschten Namen.
∴ Drei Dangling-Refs blieben zurück; jeder Aufruf mit `companion="testing-skills-with-subagents"`
liefert `COMPANION_NOT_FOUND` (`render_companion` → `companion_body` exakter String-Match,
`src/skills.rs`).

**Nicht** die Ursache: `2026-06-29-lmd-testing-anti-patterns-companion-design.md` — der
betrifft die Schwester-Skill `lmd-test-driven-development` und schließt `lmd-writing-skills`
ausdrücklich aus dem Scope aus.

## 2. Entscheidung (Auflösungsweg)

**Refs reparieren statt Alias** (gewählt). Die drei toten Verweise werden auf die
granularen `testing/*`-Namen umgebogen — wie es die Reference-Closure des Split-Specs
hätte tun müssen. Honoriert den Split, führt **keinen** Monolithen wieder ein.

Verworfen:
- **Registry-Alias** (alter Name → `testing/methodology`-Const): hielte den wörtlichen
  Aufruf am Leben, liefert aber nur die Methodik-Teilmenge unter einem Namen, der „volle
  Subagent-Test-Doku" suggeriert — irreführend, und re-etabliert den toten Namen als API.
- **Echtes Aggregat** (alter Name → zusammengesetzter Voll-Inhalt): bräuchte drei neue
  `@include`-Builtins oder einen kombinierten Seed → Inhalts-Duplikation + Fidelity-Last,
  direkt gegen die Split-Intention. (`@include` löst nur Builtins oder on-disk
  `<name>.lmd.md` unter `jail_root` auf — `fragments.rs::resolve` —, nicht die
  eingebetteten Companion-Seeds untereinander; ein Aggregat wäre also nicht „kostenlos".)

## 3. Ref-Mapping (deterministisch aus dem umgebenden Text)

| Stelle | Umgebender Text (verbatim) | → Ziel-Companion |
|---|---|---|
| `body.lmd.md:56` | „deployment checklist (see companion …) is mandatory for EACH skill" | `testing/creation-checklist` |
| `body.lmd.md:86` | „For the full testing methodology render the companion: …" | `testing/methodology` |
| `companions/bulletproofing.lmd.md:64` | „Capture rationalizations from baseline testing (render the companion: …)" | `testing/methodology` |

Jede Zuordnung folgt zwingend aus der Prosa der Stelle selbst — keine Interpretation offen.
Form der Edits:
- `body.lmd.md:56` ist Prosa (`companion "testing-skills-with-subagents"`) → Name ersetzen,
  Satzbau unverändert.
- `body.lmd.md:86` und `bulletproofing.lmd.md:64` sind `ctx_md_render(skill="…",
  companion="…")`-Aufrufe → nur das `companion="…"`-Argument ersetzen.

## 4. Komponenten

### 4.1 Seed-Edits (3 Stellen)

Reine Textersetzung in den on-disk-Seeds. Da `src/skills.rs` die Bodies via `include_str!`
einbettet, zieht der Recompile die Änderung automatisch; das bestehende
**Fragment-Consistency-Gate** (built-in == on-disk Seed) bleibt grün, weil Datei und
Einbettung dieselbe Quelle sind.

### 4.2 Dangling-Ref-Guard (neuer Test, TDD)

Ein Regressions-Test, der **alle** eingebetteten Seeds nach Companion-Verweisen scannt und
deren Auflösbarkeit erzwingt — die strukturelle Lücke, die der Split-Spec hinterließ.

- **Scan-Korpus:** alle Skill-Bodies (`all_skill_bodies()`) **plus** alle Companion-Bodies
  (`COMPANIONS`-Tabelle, dritte Spalte).
- **Erfasste Formen:**
    1. Render-Call: `companion="<name>"` (z. B. `body.lmd.md:86`, `bulletproofing.lmd.md:64`).
    2. Prosa: `companion "<name>"` (z. B. `body.lmd.md:56`).
- **Assertion:** jeder erfasste `<name>` muss als Companion in der `COMPANIONS`-Registry
  existieren (`COMPANIONS.iter().any(|(_, c, _)| c == name)`). Wo im selben Aufruf ein
  `skill="<s>"` vorausgeht, skill-scoped prüfen (`companion_body(s, name).is_some()`); sonst
  name-exists-in-COMPANIONS. Fehlt die Auflösung → Test rot mit dem konkreten toten Namen.
- **Verortung:** `src/skills.rs` `#[cfg(test)]` (neben `companion_body`/`render_companion`).

**TDD-Reihenfolge:** Test zuerst (RED — schlägt gegen die heutigen drei toten Refs an,
nennt `testing-skills-with-subagents`), dann die 3 Seed-Edits (GREEN). `cargo nextest run`.

## 5. Tests

1. **Dangling-Ref-Guard (§4.2):** RED vor den Edits; GREEN danach. Deckt alle drei Stellen
   in einem Test ab.
2. **Companion-Render der Ziele:** `companion="testing/methodology"` und
   `companion="testing/creation-checklist"` rendern erfolgreich (Marker-Assert) — bestätigt,
   dass die neuen Ziele real auflösen (Bestandsschutz, ggf. schon vorhanden).
3. **Fragment-Consistency-Gate:** unverändert grün (Seeds = include_str!-Quelle).

## 6. Determinismus & Quality Bar (#498)

- Reine Textersetzung — keine Timestamps/Counter/Random im Output-Body.
- Embedded Seed byte-identisch zur on-disk-Datei (Gate green).
- Zero clippy warnings, alle Tests grün; `cargo fmt` vor jedem `git add`.

## 7. Scope-Abgrenzung

- **In diesem Spec:** 3 Seed-Ref-Edits (`body.lmd.md` ×2, `bulletproofing.lmd.md` ×1) +
  Dangling-Ref-Guard-Test.
- **Nicht hier:** kein Alias/Aggregat des alten Namens (bewusst verworfen, §2); keine
  Änderung an `render_companion`/`companion_body`/Registry-Schema; keine `phase="…"`-Guard
  (dieselbe Mechanik wäre erweiterbar, aber der reale Bug betrifft Companions — YAGNI);
  Doc-Specs, die den alten Namen historisch nennen, bleiben unangetastet.
- **Optionale Folge (separat):** `phase="…"`-Auflösbarkeit in denselben Guard aufnehmen,
  falls künftig Phasen umbenannt werden.
