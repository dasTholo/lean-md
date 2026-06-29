# lmd-writing-skills — Voll-Port von superpowers:writing-skills (Design Spec)

**Datum:** 2026-06-29
**Branch:** `feat-lmd-v2` (direkt, keine Worktrees)
**Status:** Design — genehmigt, bereit für `writing-plans`

## Ziel

`superpowers:writing-skills` **vollständig und verlustfrei** als native lean-md-Skill
`lmd-writing-skills` adaptieren. Übergeordnetes Ziel: lean-md trägt die Skill
eigenständig — **superpowers wird für sie entbehrlich** (Reference-Closure: alle
Querverweise zeigen auf lmd-/lean-md-Ziele, nie zurück nach superpowers).

Das Muster ist die Schwester-Skill `lmd-test-driven-development`
(`content/skills/lmd-test-driven-development/`): dünner `SKILL.md`-Discovery-Stub +
schwerer, phasen-isolierter `body.lmd.md`, alles `include_str!`-embedded, gerendert
über `ctx_md_render(skill, phase|companion)`.

## Leitprinzipien

1. **Fidelity (kein Verlust).** Jede Sektion, Tabelle, Rationalization-Zeile, jeder
   Checklist-Punkt und jede Begleitdatei des Originals wird verbatim oder treu in
   genau ein lmd-Ziel (Phase / Companion / Asset) überführt. Nichts wird
   „wegzusammengefasst" oder gedroppt.
2. **lean-md-Mechanik nutzen.** Original-Aufbau eng spiegeln, aber Phasen-Rendering,
   Companions und Asset-Materialisierung von lean-md einsetzen statt einer flachen
   Monolith-Datei.
3. **Reference-Closure.** Alle Querverweise werden auf lmd-/lean-md-Ziele umgebogen.
4. **Determinismus (#498).** Tool-Output ist deterministische Funktion von (Inhalt,
   Mode, CRP, Task) — keine Timestamps/Counter/Random. Embedded Seeds sind
   byte-identisch zur On-Disk-Quelle (Fragment-Consistency-Gate). CLI==MCP
   byte-identisch.

## Architektur

```
content/skills/lmd-writing-skills/
  SKILL.md                              # dünner Discovery-Stub
  body.lmd.md                           # phasen-isoliert: red / green / refactor / rationalizations
  testing-skills-with-subagents.lmd.md  # Companion
  anthropic-best-practices.lmd.md       # Companion
  persuasion-principles.lmd.md          # Companion
  skill-discovery-optimization.lmd.md   # Companion
  bulletproofing.lmd.md                 # Companion
  skill-anatomy.lmd.md                  # Companion
  flowchart-conventions.lmd.md          # Companion
  render-graphs.js                      # Asset (install-materialisiert, nicht gerendert)
content/core/
  skill-authoring-core.lmd.md           # NEU: geteiltes Trip-Wire-Fragment, @include je Phase
```

Verdrahtung in `src/skills.rs`: `include_str!`-Konstanten + Einträge in `SKILLS`
und `COMPANIONS`. COVERAGE-Zeilen in `src/availability.rs`.
Asset-Materialisierung in `src/skill_install.rs`.

### Phasen (`body.lmd.md`)

Jede Phase wird über `capture_phase_bodies` isoliert (kein Cross-Phase-Leak) und
zieht das geteilte Core-Fragment per `@include`.

- `red` → Baseline-Pressure-Szenario schreiben, Agent **ohne** Skill scheitern
  sehen, Rationalisierungen verbatim dokumentieren.
- `green` → minimale Skill schreiben, die genau diese Failures adressiert; Micro-Test
  der Formulierung gegen No-Guidance-Control; mit Skill re-testen.
- `refactor` → Schlupflöcher schließen, Rationalization-Table + Red-Flags bauen,
  „STOP before next skill"-Disziplin.
- `rationalizations` → Gegenargumente beim „Testing überspringen"-Drang
  (Common-Rationalizations-Table).

Render: `ctx_md_render(skill="lmd-writing-skills", phase="<red|green|refactor|rationalizations>")`.

### Geteiltes Core-Fragment `skill-authoring-core`

Trip-Wire, in **jede** Phase `@include`d. Trägt:
- Iron Law: **„NO SKILL WITHOUT A FAILING TEST FIRST"**
- **„Writing skills IS TDD applied to documentation"**
- TDD-Mapping-Kurzfassung + **„The Bottom Line"**
- Trip-Wire-Zeile: *„siehe `lmd-test-driven-development` für das WARUM"* (Name-Pointer,
  kein `@`-Force-Load).

**Abgrenzung:** eigenständiges Fragment, **nicht** das `test-first-core` von
`lmd-test-driven-development`. Beide sind Geschwister-Cores (paralleles Iron Law),
kein File-`@include` voneinander. Die TDD-Verbindung ist Inhalt, nicht Include.

### Companions (7)

Render: `ctx_md_render(skill="lmd-writing-skills", companion="<name>")`.
`phase` und `companion` sind mutually exclusive.

| Companion | Quelle im Original |
|---|---|
| `testing-skills-with-subagents` | `testing-skills-with-subagents.md` (Voll-Port) + Sektion „Testing All Skill Types" + „Skill Creation Checklist" |
| `anthropic-best-practices` | `anthropic-best-practices.md` (Voll-Port) |
| `persuasion-principles` | `persuasion-principles.md` (Voll-Port) |
| `skill-discovery-optimization` | Sektion „Skill Discovery Optimization (SDO)" + „Discovery Workflow" |
| `bulletproofing` | Sektion „Bulletproofing Skills Against Rationalization" + „Match the Form to the Failure" |
| `skill-anatomy` | „What is a Skill", „Skill Types", „Directory Structure", „SKILL.md Structure", „Code Examples", „File Organization", „Anti-Patterns" |
| `flowchart-conventions` | Sektion „Flowchart Usage" + `graphviz-conventions.dot` (verbatim ```dot-Block erhalten) |

### Asset: `render-graphs.js`

`.js` ist Text → `include_str!`-embedded. `skill install` materialisiert es nach
`.claude/skills/lmd-writing-skills/render-graphs.js` (neuer Asset-Schritt in
`skill_install.rs`, idempotent/absent-only nach Vorbild
`seeds.rs::materialize_contracts`). Laufzeit-Deps (node + graphviz) liegen beim
Nutzer — wie im Original. Kein Rendering durch `ctx_md_render`.

### SKILL.md-Stub

- Frontmatter: `name: lmd-writing-skills` /
  `description: Use when creating new skills, editing existing skills, or verifying skills work before deployment`
  (1:1 Original).
- Body: Overview + Iron Law + Phasen-Pointer + Companion-Pointer +
  `**REQUIRED BACKGROUND:** lmd-test-driven-development` (Name-Pointer, kein `@`) +
  Hinweis „nie von Disk lesen — immer via `ctx_md_render`".
- Flowchart-Conventions stehen **nicht** inline, sondern im gleichnamigen Companion.

## Reference-Closure (superpowers-Unabhängigkeit)

| Original-Verweis | lmd-Ziel |
|---|---|
| `superpowers:test-driven-development` (REQUIRED BACKGROUND) | `lmd-test-driven-development` |
| „Personal skills live in your runtime's skills directory / claude-code-tools.md …" | lean-md-Mechanismus: `skill install` → `.claude/skills/` |
| `testing-skills-with-subagents.md` | Companion `testing-skills-with-subagents` |
| `persuasion-principles.md` | Companion `persuasion-principles` |
| `anthropic-best-practices.md` | Companion `anthropic-best-practices` |
| `graphviz-conventions.dot` | Companion `flowchart-conventions` |
| `render-graphs.js` | Asset (install-materialisiert) |

## Fidelity-Coverage-Matrix (Audit-Artefakt)

Vollständige Quell-Inventur — jede Original-Sektion/Datei → genau ein lmd-Ziel:

| Original-Element | lmd-Ziel |
|---|---|
| Overview, „TDD Mapping for Skills"-Table, „The Iron Law", „The Bottom Line" | `skill-authoring-core` + Stub |
| „RED-GREEN-REFACTOR for Skills", „Micro-Test Wording Before Full Scenarios" | Phasen `red`/`green`/`refactor` |
| „Common Rationalizations for Skipping Testing"-Table, „STOP: Before Moving to Next Skill" | Phase `rationalizations` (+ `refactor`) |
| „What is a Skill", „Skill Types", „Directory Structure", „SKILL.md Structure", „Code Examples", „File Organization", „Anti-Patterns" | Companion `skill-anatomy` |
| „Skill Discovery Optimization (SDO)", „Discovery Workflow" | Companion `skill-discovery-optimization` |
| „Bulletproofing Skills Against Rationalization", „Match the Form to the Failure" | Companion `bulletproofing` |
| „Testing All Skill Types", „Skill Creation Checklist (TDD Adapted)" | Companion `testing-skills-with-subagents` |
| „Flowchart Usage" + `graphviz-conventions.dot` | Companion `flowchart-conventions` |
| `anthropic-best-practices.md` | Companion `anthropic-best-practices` |
| `persuasion-principles.md` | Companion `persuasion-principles` |
| `render-graphs.js` | Asset |
| `when_flowchart` / Prozess-Diagramme | erhalten im jeweiligen Ziel (verbatim ```dot) |

## Validierung (TDD des Plans)

### Rust / nextest

- `skill_registered` — `all_skill_bodies`/`skill_body` enthalten `lmd-writing-skills`.
- `fragment_consistency` — built-in `skill-authoring-core` == On-Disk-Seed (byte-stabil).
- `phase_isolation` — keine Phase leakt Inhalt einer anderen (`red`∩`green`∩`refactor`∩`rationalizations`).
- `companion_render` — alle 7 Companions lösen nicht-leer auf; `phase`+`companion` mutually exclusive.
- `cli_eq_mcp` — `render_skill`/`render_companion` byte-identisch über CLI- und MCP-Surface.
- `coverage_rows` — `availability.rs` trägt `lmd-writing-skills`-Zeilen (Workflow-Schritt → Direktive → lean-ctx-Backing) inkl. Companion-Zeile.
- `asset_materialization` — `install_skill` schreibt `render-graphs.js`, idempotent (absent-only), korrektes Ziel.

### Subagent-Pressure-Test (Iron Law der Skill selbst)

- **RED-Baseline:** Subagent **ohne** die Skill bekommt die Aufgabe, eine Skill zu
  schreiben → Rationalisierungen/Failures verbatim dokumentieren.
- **GREEN:** gleicher Lauf **mit** gerendertem `lmd-writing-skills` → Agent compliant
  (schreibt Test zuerst, kein Verlust, Bulletproofing angewandt).

### Fidelity-Audit (kein Verlust prüfbar)

- Rust-Test: jede Phase + jeder Companion + Asset rendert nicht-leer.
- Manueller Section-by-Section-Abgleich Original ↔ Port anhand der Coverage-Matrix
  (jede Quell-Sektion abgehakt).

## Non-Goals

- **Keine native SVG-Render-Surface in lean-md.** `.dot`-Blöcke pipet der Mensch via
  `render-graphs.js` (jetzt mitgeliefert) durch graphviz selbst.
- Keine Änderung am Render-Core (`rushdown` + `evalexpr`).
- Kein neuer lean-md-CLI-Subcommand für Graph-Rendering (optionale Zukunft, hier
  bewusst ausgeklammert).

## Globale Constraints

- Tests: immer `cargo nextest run`, nie `cargo test`. Crate standalone, Repo-Root.
- Shell: kein `&&`/`||`/`;`-Chaining — jede Invocation einzeln.
- Vor jedem `git add` je Datei: `cargo fmt`.
- No worktrees — direkt auf `feat-lmd-v2`.
- Naming: ausgeschrieben `lmd-writing-skills` (keine Acronym-Kollision mit CRP-Modi).

## Plan

TDD-strukturiert, Deutsch, `docs/lean-md/plans/2026-06-29-lmd-writing-skills-port.md`,
Checkbox-Tasks im Format des Schwester-Plans
(`2026-06-29-lmd-test-driven-development-foundation.md`). Grobe Task-Sequenz:

1. SKILL.md-Stub + Registry-Eintrag (`skill_registered` RED→GREEN).
2. `skill-authoring-core`-Fragment + Fragment-Consistency-Gate.
3. `body.lmd.md`-Phasen (red/green/refactor/rationalizations) + Phasen-Isolations-Tests.
4. Companions (7) + Render-Tests + CLI==MCP.
5. Asset `render-graphs.js` + `skill_install`-Asset-Schritt + `asset_materialization`-Test.
6. COVERAGE-Zeilen in `availability.rs`.
7. Fidelity-Audit (Rust-Coverage + manueller Abgleich).
8. Subagent-Pressure-Test (RED-Baseline → GREEN).
9. Abschluss: `cargo fmt`, `cargo nextest run`, Determinismus-Gate grün.
