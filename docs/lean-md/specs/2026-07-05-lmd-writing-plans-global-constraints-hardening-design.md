# Spec: `lmd-writing-plans` härten — Global Constraints auf spec-eigene Invarianten begrenzen

- **Datum:** 2026-07-05
- **Status:** Design, genehmigt (Brainstorm)
- **Skill-Ziel:** `content/skills/lmd-writing-plans/` (Seed-Quelle) + `content/templates/plan-template.lmd.md`
- **Sprache:** Plan-/Spec-Prosa Deutsch; gewobener Seed-Content + Beispiele Englisch

## Problem

Der `lmd-writing-plans`-Skill lässt Autoren in den `## Global Constraints`-Block
eines Plans **ambient Projektregeln** schreiben, die dort weder hingehören noch
wirken. Beobachtet am Plan
`docs/lean-md/plans/2026-07-04-lmd-subagent-driven-development-port.lmd.md`
(Z. 87–111): der Block mischt

- **ambient Projektregeln** — `cargo nextest` statt `cargo test`, kein
  `&&`/`||`/`;`-Shell-Chaining, rustfmt vor `git add`, Sprach-Split
  (Englisch Code / Deutsch Prosa), und
- **spec-eigene Invarianten** — Cross-Task-Prerequisites (Bug-1-Fix vor Task 5,
  `crp: compact`), Task-Abhängigkeiten, „Seed-Verdrahtung statt -Erstellung",
  #498-Byte-Stabilität als Testgate.

Drei belegte Fehlannahmen im Skill treiben das:

1. **Faktisch falsche Zustellbehauptung.** `content/skills/lmd-writing-plans/body.lmd.md`
   (Phase `plan-format`, ~L115) sagt: „every task implicitly includes it". Empirisch
   falsch — ein `lean-md render <plan>.lmd.md --phase task-N` rendert **nur** den
   Phasen-Block, **nicht** die Meta-Head-Prosa (Goal/Architecture/Global Constraints).
   Verifiziert durch Render von `task-1` des o. g. Plans: der Brief enthält keinen
   `Global Constraints`-Abschnitt.

2. **Redundanz gegen bestehende Kanäle.** Die ambient Regeln erreichen den Implementer
   bereits über die realen Kanäle:
   - Runner/Lint/fmt → `@var test_cmd`/`lint_cmd` + Recipes (`@call gate/tdd`),
     projekt-überschrieben via `.lean-ctx/lean-md/vars.toml`
     (`test_cmd = "cargo nextest run"` gewinnt gegen den `cargo test`-Seed-Default).
   - Subagent-Verhalten (Shell-/Sprach-Regeln) → `.lean-ctx/lean-md/dispatch-contract.ext.lmd.md`
     (Header: „Add project-specific subagent rules below. Empty by default."), das
     `@dispatch` core-first + ext an **jeden** dispatchten Subagenten komponiert.
   Sie in Global Constraints erneut abzutippen verletzt die **eigene `output_rule #2`**
   des Skills („a plan never restates context the dispatch contract re-supplies").

3. **Empfänger-Missverständnis.** `Global Constraints` ist real ein
   **Controller-/Reviewer-Artefakt**, kein Implementer-Kontext:
   - Der Controller autort den Block (bzw. rendert den Meta-Head).
   - Der Reviewer bekommt ihn nur, weil der Controller ihn im sdd-`review` **explizit
     verbatim** übergibt (`content/skills/lmd-subagent-driven-development/body.lmd.md:90`:
     „pass BASE..HEAD and the plan's Global Constraints verbatim"). Auch die
     Reviewer-Companions nutzen ihn als „lens over the entire diff".
   - Der Implementer sieht ihn **nie**.

## Ziel

Den Skill so härten, dass **künftige** Pläne den Block von vornherein korrekt
schreiben: `Global Constraints` enthält **ausschließlich spec-abgeleitete
Invarianten** (die Review-Lens), und ambient Projektregeln werden **gar nicht erst
eingetragen**, weil der Skill sie an ihre echten Kanäle verweist.

Kernprinzip: **Prävention am Autoren-Zeitpunkt**, nicht Nachräumen im Self-Review.

## Nicht-Ziele (bewusst außerhalb Scope)

- **Kein Eingriff** in bestehende Pläne (der o. g. Plan bleibt unangetastet).
- **`dispatch-contract.ext.lmd.md` nicht befüllen** — die konkreten lean-md-Regeln
  dort zu hinterlegen ist Projekt-Konfiguration, kein Skill-Härten. Der Skill
  *verweist* nur auf den ext-Slot als Ziel.
- **Kein Engine-/Renderer-Change** — reine Seed-Content-Änderung. Das
  Render-Verhalten (Meta-Head-Prosa erscheint nicht im Phasen-Render) wird
  *dokumentiert*, nicht verändert.
- **Kein Wechsel** auf `ctx_knowledge`/`ctx_rules` als Zustellkanal — die bestehenden
  Kanäle (`vars.toml`, `dispatch-contract.ext`) decken den Bedarf brief-garantiert ab.

## Lösung

Drei Seed-Änderungen, Gewicht klar auf (A) + (B) — Self-Review (C) ist nur Backstop.

### A. Phase `plan-format` — positive, enge Definition am Autoren-Zeitpunkt

In `content/skills/lmd-writing-plans/body.lmd.md`, Phase `plan-format`, den
Global-Constraints-Absatz (~L115) ersetzen. Die neue Guidance liefert vor dem
Schreiben eine geschlossene Definition:

- **Was hinein gehört** — spec-abgeleitete Invarianten: projektweite Nicht-Ziele,
  Determinismus/#498-Anforderungen als Testgate, Cross-Task-Prerequisites,
  Task-Abhängigkeiten. Das ist die Lens, gegen die der Reviewer den Diff prüft.
- **Was NICHT hinein gehört** (nicht als Löschregel, sondern als „steht bereits
  woanders", mit Zielkanal):
  - Test-Runner / Lint / fmt-Kommandos → `@var`/`.lean-ctx/lean-md/vars.toml` + Recipes.
  - Subagent-Verhaltensregeln (Shell-Chaining, Sprach-Split, Commit-Form) →
    `.lean-ctx/lean-md/dispatch-contract.ext.lmd.md`.
- **Wer den Block empfängt** (die faktische Korrektur, ersetzt „every task implicitly
  includes it"): der `--phase task-N`-Render enthält die Meta-Head-Prosa **nicht**;
  `Global Constraints` ist ein **Controller-/Reviewer-Artefakt** — der Controller
  autort es und reicht es im Review verbatim an den Reviewer weiter; ein Task erbt es
  nur, wenn er es explizit `@include`t. Daraus versteht der Autor *warum* nur
  Invarianten reingehören: er schreibt für die Review-Lens, nicht als
  Implementer-Kontext.

### B. Template `plan-template.lmd.md` — die prägende Form

Das mitgelieferte Template ist der stärkste Prägehebel (der Autor kopiert die Form).

- Die Meta-Head-Guidance (aktuell L23: „write Goal / Architecture / Global
  Constraints as prose copied from the spec") auf die enge Definition umstellen:
  Global Constraints = spec-eigene Invarianten für die Controller/Reviewer-Lens,
  **keine** ambient Projektregeln (die leben in `vars.toml` bzw.
  `dispatch-contract.ext`).
- Ein knappes **korrektes Positiv-Beispiel** eines Global-Constraints-Blocks
  einfügen (nur Invarianten, z. B. ein Nicht-Ziel + eine #498-Testgate-Zeile + eine
  Cross-Task-Prerequisite), damit die richtige Gestalt abgekupfert wird — statt der
  offenen „schreib Prosa aus dem Spec"-Einladung.

### C. Phase `self-review` #4 — auf Backstop zurückstufen

In `content/skills/lmd-writing-plans/body.lmd.md`, Phase `self-review`, den
„Ambient-context scan" (#4, ~L214) von einer Löschmechanik auf einen knappen
Konsistenz-Check umstellen: „Global Constraints enthält ausschließlich
spec-abgeleitete Invarianten? Ambient Projektregeln (Runner/Lint/fmt/Shell/Sprache)
gehören nach `vars.toml`/`dispatch-contract.ext`, nicht in den Plan." — als
Sicherheitsnetz, falls die Autoren-Regel doch durchrutscht. Das Gewicht liegt
eindeutig auf A + B.

## Betroffene Dateien

| Datei | Änderung |
|---|---|
| `content/skills/lmd-writing-plans/body.lmd.md` | Phase `plan-format`: Global-Constraints-Absatz neu (A). Phase `self-review`: #4 auf Backstop (C). |
| `content/templates/plan-template.lmd.md` | Meta-Head-Guidance + Positiv-Beispiel (B). |

## Constraints (Umsetzung)

- **#498 Byte-Stabilität:** Seeds sind `include_str!`-eingebettet; das
  Fragment-Consistency-Gate (built-in == on-disk Seed) muss grün bleiben. Änderung
  erfolgt an der on-disk Quelle in `content/…`, der eingebettete Copy folgt via
  `include_str!` automatisch — kein Byte-Drift.
- **Materialisierte Projektkopie:** nach Template-Änderung die materialisierte
  `.lean-ctx/lean-md/plan-template.lmd.md` via `force`-Refresh nachziehen (die
  Materialisierung ist absent-only; ein geänderter Seed braucht `force`).
- **Sprache:** neuer Seed-Content Englisch; diese Spec-Prosa Deutsch.
- **Keine bestehenden Tests brechen.**

## Verifikation

- Phasen-Render nach der Änderung: `lean-md render` der Phasen `plan-format` und
  `self-review` zeigt die neue Guidance korrekt (keine Macro-/Include-Fehler).
- Template-Render/Signatures unverändert lauffähig; `plan_template_self_documents`
  und die Fragment-Consistency-Tests bleiben grün.
- Voller `cargo nextest run` grün; `cargo clippy --all-targets -- -D warnings`
  warnungsfrei.

## Akzeptanzkriterien

1. Phase `plan-format` definiert `Global Constraints` positiv als „nur spec-eigene
   Invarianten" und benennt `vars.toml` + `dispatch-contract.ext` als Zielkanäle
   für ambient Regeln.
2. Die falsche Behauptung „every task implicitly includes it" ist durch die korrekte
   Empfänger-Beschreibung (Controller/Reviewer-Artefakt; Phasen-Render ohne
   Meta-Head-Prosa) ersetzt.
3. Das Template zeigt ein korrektes Positiv-Beispiel eines Invarianten-only-Blocks.
4. Self-Review #4 ist ein knapper Konsistenz-Check (Backstop), keine Löschmechanik.
5. Fragment-Consistency-Gate + volle Testsuite grün; clippy warnungsfrei.
