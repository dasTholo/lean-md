# lmd-test-driven-development — Native lean-md Port + Skill-Platform-Fundament (Design-Spec)

Status: **Design (in Review) — Reconciled gg. Original + Code-IST** · Datum: 2026-06-29 · Branch: `feat-lmd-v2`

> **Reconciliation-Hinweis (2026-06-29):** Dieses Spec wurde gegen das superpowers-Original
> (`test-driven-development/SKILL.md` + `testing-anti-patterns.md`, v6.0.3) **und** den
> bereits **teil-implementierten** Code-Stand abgeglichen. Ziel: **keine Inhalte/Möglichkeiten
> verloren**, lean-md ohne superpowers-Skill nutzbar. Geänderte Annahmen ggü. der ersten Fassung
> sind als „⟳ Reconciled" markiert.

**Position im Authoring-Layer-Port (Spec 1 von 3):**

```
test-driven-development   →   writing-skills   →   brainstorming
   (DIESER Spec, #1)            (Spec #2)            (Spec #3, Re-Anchor)
```

**Dieser Spec ist Nr. 1.** Er liefert (a) die erste nativ portierte Skill
`lmd-test-driven-development` **inkl. Companion** `testing-anti-patterns`, und (b) das geteilte
**Skill-Platform-Fundament**, das Spec #2/#3 wiederverwenden. TDD steht zuerst, weil `writing-skills`
(Spec #2) TDD als `REQUIRED BACKGROUND` voraussetzt — und das Fundament muss mit der ersten Skill landen.

**Referenz-Specs (Abgleich §9):**

- `2026-06-29-lmd-brainstorm-design.md` — Schwester-Spec (#3); liefert Architektur-Vokabular
  (Kanal ①/②, D6/D8/D10, Body-Override D7, Gates). DIESER Spec **baut** das Fundament, das jener konsumiert.
- `2026-06-26-lean-md-standalone-addon-design.md` — autoritative Baseline (zero-config, 30 Bridges,
  `skills.rs`/`seeds.rs`/`availability.rs`, lean-ctx-seitiger Skill-Installer entfernt → Install lebt in lean-md).

## 1. Ziel & Kontext

Die superpowers-`test-driven-development`-Skill wird als **nativer lean-md-Skill**
`lmd-test-driven-development` portiert: binary-embedded, **phasenweise** über
`ctx_md_render(skill, phase)` gerendert; der Companion `testing-anti-patterns` über
`ctx_md_render(skill, companion)`. Gleichzeitig wird die heute auf **eine** hartkodierte Skill
(`lmd-brainstorm`) zugeschnittene Maschinerie zu einem echten **Skill-Platform-Fundament** generalisiert.

**Warum TDD-Inhalt + Companion + Fundament in einem Spec:** Das Fundament hat **keinen** eigenen
user-sichtbaren Deliverable — es muss mit dem ersten echten Skill-Konsumenten landen. TDD ist dieser
erste Konsument, und der Companion gehört zur Skill (Original koppelt sie: SKILL.md verlinkt
`testing-anti-patterns.md`). **⟳ Reconciled:** der Companion ist damit **in Spec #1** (nicht deferred).

**Nicht-Ziel:** `writing-skills` (→ Spec #2); brainstorm-Re-Anchor (→ Spec #3); kein Eingriff in den
render core (`rushdown`/`evalexpr`).

### 1.1 Ausgangszustand (verifiziertes Code-IST — ⟳ Reconciled, war stale)

Die erste Fassung beschrieb einen Greenfield-Stand; **tatsächlich ist Spec #1 bereits teil-implementiert**:

- `content/skills/lmd-test-driven-development/` existiert mit `SKILL.md` (Stub), `body.lmd.md`
  (4 `@phase`-Blöcke red/green/refactor/rationalizations, `@var test_cmd`), `_includes/test-first-core.lmd.md`
  und `companions/testing-anti-patterns.lmd.md`. ✅
- **SKILL.md-Stub trägt bereits:** Core principle („If you didn't watch the test fail…"), When-to-Use
  (Always/Exceptions/ask human partner/„skip just this once = rationalization"), Iron Law, RGR mit
  Render-Pointern, Final Rule, Companion-Pointer mit `ctx_md_render(…, companion="testing-anti-patterns")`,
  „Pass exactly one of `phase` or `companion`". ✅ — die in der Erstfassung als „fehlend" markierten
  Sektionen sind im Stub größtenteils vorhanden.
- `content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md` existiert bereits (Spec #2-Vorarbeit).
- `content/core/_fragments/tool-quick-ref.lmd.md` existiert (entgegen Erstfassung „leer"); die
  `_fragments`→`_includes`-Umbenennung ist **nur** beim TDD-Skill vollzogen, `content/core/_fragments/`
  besteht noch.
- **🔴 Render KAPUTT:** `@include test-first-core` resolved **nicht** — das Fragment liegt als Datei
  unter `_includes/`, ist aber **nicht in `FragmentRegistry::with_builtins()` registriert**. Folge:
  Body rendert `PHASE_ABORTED` in **allen 4 Phasen**, Companion meldet `@include err: fragment not found`.
  Die Skill ist aktuell **nicht renderbar**. → Pflicht-Gate (E14, §5 Gate 2).
- **🟡 Disziplin-Inhalt überkomprimiert** ggü. Original (echter Inhaltsverlust): Common Rationalizations
  **4 statt 11**, Red Flags **4 statt 13**, „Why Order Matters" 1 Zeile statt 5, „When Stuck" 1 Zeile
  statt 4-Zeilen-Tabelle, „Debugging Integration" fehlt. → E12 (§4.1).
- `src/skills.rs` — `skill_body()` Registry-Lookup-Stand prüfen (Erstfassung: hartkodierter `match`).
- `src/bin/lean_md.rs` — `ctx_md_render`-Verdrahtung für `skill`/`phase`/`companion` prüfen
  (Companion-Param ist im Stub referenziert → muss serverseitig existieren).
- Kein verifizierter `claude_state_dir()`-Helper / `CLAUDE_CONFIG_DIR`-Handling → ggf. Neubau (R3).

## 2. Entscheidungen

| #   | Fork                       | Entscheidung |
| E1  | Reihenfolge                | **TDD zuerst.** `writing-skills` setzt TDD als `REQUIRED BACKGROUND` voraus; Fundament landet mit dem ersten Konsumenten. |
| E2  | Skill-Name                 | **`lmd-test-driven-development`** — 1:1-Spiegel des superpowers-Quellnamens. **Kollisionsgrund:** `lmd-tdd` verworfen, weil `tdd` in lean-ctx = **Terse Data Density** / CRP-Modus (`LEAN_CTX_CRP_MODE=tdd`, `CrpMode::Tdd`). Ausgeschriebener Name kollidiert nicht (grep-`tdd` trifft ihn nicht). |
| E3  | Render-Modell              | **Phasen-isoliert** (`red`/`green`/`refactor`/`rationalizations`); `ctx_md_render(phase=X)` rendert nur X. **Disziplin-Mitigation:** kompaktes `test-first-core`-Fragment via `@include` in **jede** Phase. |
| E4 ⟳| Companion-Scope            | **⟳ Reconciled: Companion IN Spec #1.** Erstfassung deferrte den Companion auf Spec #2 — der Code-IST hat ihn bereits als **gerendertes Target** (`ctx_md_render(skill, companion)`). Der Companion ist 1:1 zum Original gekoppelt (SKILL.md verlinkt ihn) → gehört zur Skill, nicht in ein Folge-Spec. **Companion-Render-Mechanik** (companion-Param im Tool-Schema + Handler-Branch) ist damit Teil des Fundaments. Eine generische **Companion-Registry-Spalte / Out-of-band-Targets für *weitere* Skills** bleibt Spec #2; hier genügt der `companion`-Param + Lookup für den einen TDD-Companion. |
| E5  | Discipline-Fragment        | **`test-first-core`** (Iron Law + „Letter==Spirit" + Core principle + kompakte Red-Flags), **skill-eigen** unter `content/skills/lmd-test-driven-development/_includes/`. **Muss als Built-in registriert werden** (flacher globaler Name; `@include` löst per Name auf — Ordner kosmetisch). Name bewusst **nicht** `tdd-core` (grep-Kollision mit `tdd_schema`/`tdd_legend`). |
| E6  | Body-Override (D7)         | `render_skill` prüft zuerst Projekt-Overlay, sonst embedded Const → lokale Phasen-Iteration ohne Recompile. |
| E7  | `skill install`-Heimat     | **In lean-md** (Baseline §2.2: lean-ctx-Installer entfernt). Opt-in = Invocation. |
| E8  | `ctx_md_render`-Verdrahtung| **Skill + Phase + Companion exponieren** (`render_skill`): `tool_defs()`-inputSchema (`skill`, `phase`, `companion`, **genau eines von `phase`/`companion`**) + `tools/call`-Branch + CLI `render --skill/--phase/--companion`. Ohne dies ist keine native Skill invocable. |
| E9  | `COVERAGE`-Dimension       | `(step, directive, backing)` → **`(skill, step, directive, backing)`**; Audit-Doc bekommt `lmd-test-driven-development`-Abschnitt (inkl. Companion-Zeile, vgl. Commit `feat(availability): COVERAGE row for testing-anti-patterns companion`); stale Pfad `rust/src/lmd/availability.rs` → `src/availability.rs` fixen. |
| E10 | Keyword-Coverage           | `description`/Body schreiben **„test-driven development" / „test-first"** aus; das Acronym **„TDD" nur disambiguiert** im Body (z. B. „TDD (test-driven development)"), nie als nacktes Keyword (CRP-`tdd`-Verwechslungsschutz). |
| E11 | Install-**Scope**          | **Auswählbar, Default `--local`.** `--local` → `<project_root>/.claude/skills/<name>/` (env-unabhängig, versionierbar). `--global` → `claude_state_dir()/skills/<name>/` (`CLAUDE_CONFIG_DIR` sonst `~/.claude`). `CLAUDE_CONFIG_DIR` betrifft **nur** das globale Ziel. **Companion-Datei** wird im selben Schritt mit-materialisiert (Discovery via relativem Link bleibt als Fallback erhalten, auch wenn der Primärweg `ctx_md_render(companion=…)` ist). |
| E12 ⟳| Disziplin-Inhalt (Mittelweg)| **⟳ Reconciled — Fidelity-Restore (Mittelweg).** Überkomprimierte **Disziplin-Trip-Wires voll wiederherstellen**: **Common Rationalizations (alle 11 Zeilen)** + **Red Flags (alle 13)** + **When-Stuck-Tabelle (4 Zeilen)** + **Debugging Integration** („Never fix bugs without a test"). **YAGNI weggelassen:** Good-Tests-Tabelle + dedizierter Bug-Fix-Walkthrough (der RGR-Zyklus im Body deckt das ab). Begründung: diese Tabellen sind kein Beispiel-Ballast, sondern der **Wirkmechanismus** der Skill; ihr Fehlen entschärft die Disziplin → widerspräche dem Ziel „ohne superpowers arbeitbar". |
| E13 ⟳| Beispiel-Politik (code-frei)| **⟳ Reconciled.** **Keine Codeblöcke** in Body/Companion. Original-TS-Beispiele (retry, bug-fix, mock-Snippets) werden **nicht** portiert — Claude folgt der TDD-Disziplin ohne verbose Listings. Body nutzt knappe Prosa-Good/Bad; Companion bleibt reine Prosa + Gate-Functions (Code-IST erfüllt das bereits). Antwort auf „braucht Claude die Beispiele?": **Code-Beispiele nein, Disziplin-Tabellen ja** (E12). |
| E14 ⟳| Render-Korrektheit (Pflicht-Gate)| **⟳ Reconciled — Bugfix verankert.** `test-first-core` **in `FragmentRegistry::with_builtins()` registrieren** (flacher globaler Name). Pflicht-**Render-Gate**: alle 4 Phasen **und** der Companion rendern ohne `PHASE_ABORTED`/`@include err`; Iron-Law-Marker in jeder isolierten Phase präsent. Behebt den aktuellen Komplett-Render-Ausfall (§1.1). |

Aus brainstorm-Spec **übernommen** (nicht neu entschieden): D6 (Materialisierung Schicht A global / B Projekt-Overlay), D8 (Overlay-Pfad `.lean-ctx/lean-md/`), D10 (Opt-in = Invocation).

## 3. Architektur

### 3.1 Zwei Schichten

**Inhalt (die Skill):** `lmd-test-driven-development` = 4-Phasen-Body + Companion `testing-anti-patterns`,
jede Phase `@include test-first-core`.

**Fundament (geteilt, von Spec #2/#3 wiederverwendet):**

1. **Skill-Registry** — `SKILLS: &[(&str, &str)]`-Tabelle statt hartkodiertem `match`.
2. **Body-Override (D7)** — Projekt-Overlay-Auflösung in `render_skill`.
3. **`ctx_md_render`-Verdrahtung** — `render_skill` über MCP-Tool + CLI exponiert, **mit `companion`-Param** (E8).
4. **`skill install`/`remove`** — Materialisierung von `SKILL.md` **+ Companion** nach Scope-Ziel (E11).
5. **`COVERAGE` mit `skill`-Dimension** — Audit-Doc + Gate.
6. **Fragment-Registrierung** — `_includes/`-Partials als Built-ins in `FragmentRegistry` (E5/E14).

### 3.2 Seed-Layout (alles unter `content/`, alles embedded — ⟳ entspricht Code-IST)

```
content/skills/lmd-test-driven-development/
  SKILL.md                         ~ Stub: frontmatter + Core principle + When-to-Use + Iron Law +
                                     RGR-Render-Pointer + Final Rule + Companion-Pointer; via skill install rausgeschrieben
  body.lmd.md                      ~ 4 @phase-Blöcke (red/green/refactor/rationalizations), @var test_cmd; via ctx_md_render
  _includes/
    test-first-core.lmd.md         + Iron Law + Core principle + "Letter==Spirit" + kompakte Red-Flags;
                                     @include in jede Phase; ALS BUILT-IN REGISTRIEREN (E5/E14)
  companions/
    testing-anti-patterns.lmd.md   ~ 5 Anti-Patterns + Gate-Functions + Quick-Ref + Red Flags; code-frei;
                                     via ctx_md_render(skill, companion="testing-anti-patterns")
```

**`_includes/` vs `companions/`:** `_includes/` = `@include`-Partials (kein eigenes Render-Target,
Unterstrich-Konvention, flacher globaler Fragment-Namespace). `companions/` = eigenständige
**Render-Targets** via `companion`-Param (E4/E8). `content/core/` bleibt skillübergreifenden Fragmenten
(`hard-rules`, `dispatch-contract`) vorbehalten.

### 3.3 Materialisierung & Auflösung (D6/D7 + Companion)

| Seed | Laufzeit-Ziel | Scope |
| `_includes/test-first-core.lmd.md` | bleibt im Binary (Built-in, flacher Name) | embedded/locked |
| `SKILL.md` | **`--local`** (Default): `<project>/.claude/skills/<n>/` · **`--global`**: `<CLAUDE_CONFIG_DIR\|~/.claude>/skills/<n>/` | materialisiert |
| `body.lmd.md` | bleibt im Binary **— oder Projekt-Overlay (D7)** | embedded, override-fähig |
| `companions/testing-anti-patterns.lmd.md` | embedded → via `ctx_md_render(companion=…)`; **zusätzlich** beim install mit-materialisiert (relativer-Link-Fallback) | embedded, gerendert |

**Auflösungs-Reihenfolge (D7):** `render_skill()` prüft zuerst
`<jail_root>/.lean-ctx/lean-md/skills/<n>/body.lmd.md` (jailed); existiert → Overlay, sonst der `include_str!`-Const.

**Zwei Pfade, zwei Jobs (Discovery vs. Render — NICHT verwechseln):**

| | Discovery (Claude Code findet die Skill) | Render (lean-md-intern) |
|---|---|---|
| Pfad/Weg | `.claude/skills/<n>/SKILL.md` (E11) | `ctx_md_render(skill, phase\|companion)` (E8) |
| Wer liest | die **Harness** → zeigt `/lmd-…` | **`render_skill`** beim Tool-Aufruf |
| Inhalt | dünner **Stub** | Phasen-Body **oder** Companion, aus embedded Const / D7-Overlay |

Der schwere Body **und** der Companion fließen über `ctx_md_render` (Kanal ①); der Stub (Kanal ②) erfüllt
nur Claude Codes Discovery-Anforderung. Der D7-Override-Pfad ist **kein** Discovery-Pfad.

## 4. Komponenten

### 4.1 Body — die 4 Phasen (treuer, code-freier Port; E12/E13)

`@phase "name" … @phase-end`; `ctx_md_render(phase=X)` rendert nur Block X. Jede Phase startet mit
`@include test-first-core`. `@var test_cmd` (Default `cargo test`, Projekt-Hinweis: `cargo nextest run`)
parametrisiert den Test-Runner.

| Phase | Inhalt (aus superpowers-`test-driven-development`, code-frei) | `@include` |
| `red` | failing test first; **Verify RED (mandatory)** via `ctx_shell ({{ var test_cmd }})` — Test muss korrekt fehlschlagen (Assertion, nicht Compile-Error); knappe Good/Bad-**Prosa** | `test-first-core` |
| `green` | minimaler Code zum Bestehen; **Verify GREEN (mandatory)**; YAGNI | `test-first-core` |
| `refactor` | Aufräumen **nur unter grün** (Duplikate/Namen/Helper); keine neue Behavior → sonst zurück zu RED | `test-first-core` |
| `rationalizations` | **volle „Common Rationalizations"-Tabelle (alle 11 Zeilen)** + **„When Stuck"-Tabelle (4 Zeilen)** + **„Debugging Integration"** („Never fix bugs without a test") + Verification-Checklist + Companion-Pointer | `test-first-core` |

**⟳ E12-Restore (rationalizations-Phase):** Die volle 11-Zeilen-Rationalizations-Tabelle
(Excuse\|Reality) ersetzt die aktuelle 4-Zeilen-Kurzfassung. Die 4-Zeilen-„When Stuck"-Tabelle
(Don't-know-how-to-test / Test-too-complicated / Must-mock-everything / Test-setup-huge) und der
„Debugging Integration"-Satz werden ergänzt. **YAGNI weggelassen:** Good-Tests-Tabelle + dedizierter
Bug-Fix-Walkthrough.

**`test-first-core`-Fragment (kompakt, in jede Phase):** Iron Law
(`NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST` + „Delete means delete"), **Core principle**
(„If you didn't watch the test fail, you don't know if it tests the right thing"), der Satz
**„Violating the letter of the rules is violating the spirit of the rules."**, und die **volle
13-Punkt-Red-Flags-Liste** (⟳ E12: von 4 auf 13 erweitert). So trägt **jede** isoliert gerenderte
Phase die Disziplin-Trip-Wires.

**lean-ctx-Adaption:** Test-Runner **Rust/`cargo nextest run`** statt TS/`npm test` (Host-Crate +
Hardrule: nie `cargo test`, kein `&&`-Chaining, `ctx_shell` statt bash). „Verify RED/GREEN" referenziert
`ctx_shell ({{ var test_cmd }})`. **Keine Codeblöcke** (E13).

### 4.2 SKILL.md-Stub (SDO-konform — entspricht Code-IST)

```markdown
---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---
```

`description` = **nur Trigger**, kein Workflow-Summary (writing-skills SDO). Body des Stubs trägt
Core principle, „Letter==Spirit", When-to-Use (Always/Exceptions/„ask human partner"/„skip just this once"),
Iron Law, RGR-Render-Pointer, **Final Rule**, Companion-Pointer (`ctx_md_render(…, companion="testing-anti-patterns")`,
„Pass exactly one of `phase` or `companion`"). Keyword-Coverage im Body, Acronym „TDD" nur disambiguiert (E10).

### 4.3 Skill-Registry (`src/skills.rs`)

`skill_body()` → Lookup gegen `SKILLS: &[(&str, &str)]` mit `lmd-brainstorm` (bestehend) und
`lmd-test-driven-development` (neu). Unbekannt → `None`.

### 4.4 Body-Override (`render_skill`, D7)

Vor dem embedded Const: jailed Read von `<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md`;
existiert → Overlay-Quelle, sonst Const. PathJail-gebunden (Spec §6).

### 4.5 `ctx_md_render`-Verdrahtung (E8 — inkl. Companion)

- `tool_defs()`: `ctx_md_render`-inputSchema um `skill`, `phase`, **`companion`** (alle optional;
  **genau eines** von `phase`/`companion`) erweitern.
- `tools/call`-Handler: `skill`+`phase` → `render_skill(skill, phase, …)`; `skill`+`companion` →
  Companion-Render; sonst bestehender `do_render`-Pfad. Fehler `UnknownSkill`/`PhaseNotFound`/
  `UnknownCompanion`/beide-gesetzt → JSON-RPC `-32602`.
- CLI: `lean-md render --skill <name> [--phase <p> | --companion <c>]`.
- **Byte-stabil (#498):** CLI- und MCP-Pfad rufen denselben `render_skill` → identisches Ergebnis.

### 4.6 Fragment-Registrierung (`src/fragments.rs`, E5/E14 — 🔴 Bugfix)

`test-first-core` (und ggf. weitere `_includes/`-Partials) in `FragmentRegistry::with_builtins()`
als Built-in registrieren (flacher globaler Name; `include_str!`-Quelle = Seed-Datei). **Behebt den
aktuellen `@include err: fragment not found`-Render-Ausfall** in Body (alle Phasen) und Companion.
Fragment-Consistency-Gate auf die neue Registrierung erweitern (built-in == on-disk).

### 4.7 `skill install`/`remove` (`src/skill_install.rs`)

- **Scope (E11):** `install|remove <name> [--global|--local]`, Default `--local`
  (`<project_root>/.claude/skills/<name>/`); `--global` → `claude_state_dir()/skills/<name>/`
  (neuer `claude_state_dir()`-Helper: `CLAUDE_CONFIG_DIR` sonst `~/.claude`; **nur** das globale Ziel
  reagiert auf die Env).
- `install`: `SKILL.md` **+ `companions/testing-anti-patterns.lmd.md`** (relativer-Link-Fallback) → Ziel-Dir.
  Atomic, idempotent. `remove`: entfernt nur das lmd-eigene Skill-Dir im gewählten Scope.
- CLI-Wiring: `"skill"`-Arm in `bin/lean_md.rs::main()` → `cmd_skill`.

### 4.8 `COVERAGE` + Audit-Doc (E9)

- `availability.rs`: `COVERAGE` → `(skill, step, directive, backing)`; bestehende Zeilen `skill="lmd-brainstorm"`.
  Neu: `lmd-test-driven-development`-Zeilen (direktiv-arm) + **Companion-Zeile** (vgl. bestehender Commit).
  Test-Execution (`ctx_shell "cargo nextest run"`) ist **keine** registrierte Direktive → Gap-Liste mit
  Begründung „TDD ist Prosa-Disziplin, Test-Run via roher `ctx_shell`".
- `content/tooling/availability-audit.md`: `lmd-test-driven-development`-Abschnitt; stale Pfad fixen.

## 5. Wiring & Gates

**Wiring:** `src/skills.rs` (Registry + Body-Override), `src/skill_install.rs` (+ `lib.rs`/`mod.rs`-Export),
`src/bin/lean_md.rs` (`ctx_md_render` skill/phase/**companion** + `skill`-Subcommand), `src/availability.rs`
(skill-Dim), `src/fragments.rs` (**`test-first-core` als Built-in registrieren** + Consistency-Gate),
`content/skills/lmd-test-driven-development/*` + Audit-Doc.

**Gates (`cargo nextest run`, nie `cargo test`):**

1. **Phasen-Isolation** für alle 4 Phasen (`red`/`green`/`refactor`/`rationalizations`) — kein Cross-Phase-Leak.
2. **🔴 Render-Gate (E14):** alle 4 Phasen **und** der Companion rendern ohne `PHASE_ABORTED`/`@include err`;
   `@include test-first-core` resolved in **jeder** Phase (Iron-Law-Marker präsent). Behebt §1.1-Ausfall.
3. **Registry**: `skill_body()` löst `lmd-brainstorm` **und** `lmd-test-driven-development`; unbekannt → `None`.
4. **Body-Override (D7)**: Overlay-Datei vorhanden → Overlay gerendert; absent → embedded (tempdir + jail).
5. **`skill install`-Roundtrip beide Scopes** (tempdir): `--local` → `<tmp_project>/.claude/skills/<name>/`
   (ignoriert gesetztes `CLAUDE_CONFIG_DIR`); `--global` mit `CLAUDE_CONFIG_DIR`-Pin → `<pin>/skills/<name>/`.
   **`SKILL.md` + Companion** da; remove → weg; idempotent.
6. **`ctx_md_render` skill/phase/companion**: MCP- und CLI-Pfad rendern identisch (byte-stabil);
   `phase` **und** `companion` gleichzeitig → `-32602`; unbekannte → `-32602`.
7. **Fragment-Consistency-Gate** grün: built-in == on-disk für alle Seeds (body, test-first-core, SKILL.md, Companion).
8. **`COVERAGE`↔Audit-Doc** inkl. `skill`-Dimension (+ Companion-Zeile); jede covered Direktive in `default_registry()`.
9. **Determinismus (#498)**: keine Timestamps/Counter, byte-stabil; `CliBackend` == `McpBackend`.
10. **⟳ Fidelity-Gate (E12):** `rationalizations`-Phase enthält **alle 11** Rationalizations-Zeilen;
    `test-first-core` enthält **alle 13** Red Flags; When-Stuck-Tabelle (4) + Debugging-Integration präsent.
    (Marker-/Zeilen-Count-Assert gegen Original.)

## 6. Lokaler Test-Flow (in-Repo, kein globaler Eingriff)

1. **Bugfix zuerst:** `test-first-core` registrieren (E14) → Render-Gate (2) grün.
2. Disziplin-Inhalt restaurieren (E12): Rationalizations 11, Red Flags 13, When-Stuck, Debugging-Integration.
3. Registry + Body-Override + `skill_install` (+ Companion) + `ctx_md_render`-companion-Wiring + `COVERAGE`.
4. `cargo nextest run --manifest-path Cargo.toml` → alle 10 Gates grün.
5. `cargo fmt` vor jedem `git add`.
6. Manueller E2E: `lean-md skill install lmd-test-driven-development --local` → repo-lokales
   `.claude/skills/lmd-test-driven-development/` (SKILL.md + Companion);
   `lean-md render --skill lmd-test-driven-development --phase red` und
   `--companion testing-anti-patterns` rendern beide ohne Abort.
7. `.lean-ctx/lean-md/` + repo-lokales `.claude/` in `.gitignore` (falls noch nicht).

## 7. Risiken & offene Punkte

- **R1 — Disziplin-Schwäche durch Phasen-Isolation:** mitigiert durch `test-first-core`-`@include` (Gate 2)
  **mit voller 13-Punkt-Red-Flags-Liste** (E12). Restrisiko: ein Consumer rendert nur `green` und sieht die
  volle Rationalizations-Tabelle nicht — akzeptiert, da Red-Flags via Fragment präsent.
- **R2 — Port-Treue vs. net-neuer Skill (writing-skills Iron Law):** Port einer upstream pressure-getesteten
  Skill; Port-Risiko = Treue + Render-Korrektheit (§5-Gates 2/10). **Bootstrap:** Spec #1 wird mit der
  *superpowers*-`writing-skills` autorisiert (native `lmd-writing-skills` erst nach Spec #2).
- **R3 — `claude_state_dir()` Neubau:** muss `CLAUDE_CONFIG_DIR` korrekt honorieren; Test pinnt die Env (Gate 5).
- **R4 ✅ — Companion (gelöst):** Erstfassung deferrte `testing-anti-patterns.md` auf Spec #2; **⟳ jetzt in
  Spec #1** als gerendertes Target (E4), code-frei portiert (E13). Generische Companion-Registry für *weitere*
  Skills bleibt Spec #2.
- **R5 — Naming-Konvention-Drift:** `lmd-brainstorm` ist von `brainstorming` verkürzt (pre-existing); neue
  Konvention (E2) verlangt 1:1-Spiegelung. Vermerkt, **nicht** Teil dieses Specs (kein Rename hier).
- **R6 ⟳ — `_fragments`/`_includes`-Drift:** TDD-Skill nutzt `_includes/`, `content/core/_fragments/` besteht
  weiter (Code-IST). Vereinheitlichung nicht Teil von Spec #1 (kein Rename von Bestandsfragmenten); vermerkt.

## 8. Scope-Abgrenzung

- **In Spec #1:** `lmd-test-driven-development` (4 Phasen + `test-first-core`) **+ Companion
  `testing-anti-patterns`** (gerendertes Target), Fragment-Registrierung (🔴 Bugfix), Skill-Registry,
  Body-Override D7, `ctx_md_render` skill/phase/companion-Verdrahtung, `skill install/remove`
  (+ Companion-Materialisierung) + `claude_state_dir()`, `COVERAGE` skill-Dimension + Audit-Doc,
  **Fidelity-Restore** (E12), alle 10 Gates.
- **→ Spec #2:** `lmd-writing-skills`; **generische** Companion-Registry-Spalte / Out-of-band-Targets
  für *weitere* Skills (nicht der eine TDD-Companion).
- **→ Spec #3:** `lmd-brainstorm`-Re-Anchor auf native Konsumption.
- **Verworfen (YAGNI):** `Skill`-Trait-Abstraktion; Phantom-„Infra-only"-Spec; **Original-Code-Beispiele
  (TS retry/bug-fix/mock-Snippets)** (E13); **Good-Tests-Tabelle + dedizierter Bug-Fix-Walkthrough** (E12).

## 9. Abgleich mit Original + Referenz-Specs (Vollständigkeits-Check)

**Original `test-driven-development/SKILL.md` → Verortung (⟳ vollständig abgeglichen):**

| Original-Abschnitt | Verortung |
| Overview / Core principle / „Letter==Spirit" | ✅ `test-first-core` + SKILL.md-Stub |
| When to Use (Always/Exceptions/ask human partner/„skip just this once") | ✅ SKILL.md-Stub (§4.2) |
| Iron Law (+ „Delete means delete") | ✅ `test-first-core` |
| Red-Green-Refactor (Zyklus) | ✅ §4.1 Phasen + RGR-Render-Pointer im Stub (dot-Digraph als Render-Pointer ersetzt) |
| RED/GREEN/REFACTOR + Verify (mandatory) | ✅ §4.1 (code-frei, Prosa-Good/Bad, E13) |
| Good Tests (Tabelle) | ⟳ **bewusst weggelassen** (E12 YAGNI) |
| Why Order Matters | ✅ abgedeckt durch volle Rationalizations-Tabelle (E12) |
| Common Rationalizations (11) | ✅ **voll restauriert** rationalizations-Phase (E12) |
| Red Flags (13) | ✅ **voll restauriert** `test-first-core` (E12) |
| Example: Bug Fix (Walkthrough) | ⟳ **bewusst weggelassen** (E12 YAGNI; RGR-Zyklus deckt ab) |
| Verification Checklist | ✅ rationalizations-Phase |
| When Stuck (Tabelle, 4) | ✅ **restauriert** (E12) |
| Debugging Integration | ✅ **restauriert** (E12) |
| Final Rule | ✅ SKILL.md-Stub |
| Testing Anti-Patterns (Pointer) | ✅ Pointer im Stub + Companion gerendert (E4) |

**Companion `testing-anti-patterns.md` → `companions/testing-anti-patterns.lmd.md`:**
5 Anti-Patterns + Iron Laws + Gate-Functions + Quick-Reference + Red Flags + Bottom Line — ✅ vollständig,
**code-frei** (E13). Gerendert via `ctx_md_render(skill, companion="testing-anti-patterns")` (E4/E8).

**Referenz-Specs:**

| Aspekt | Berücksichtigt? |
| `ctx_md_render(skill, phase\|companion)` render-on-invoke | ✅ §4.5 (companion-Param ergänzt, E8) |
| Phasen-Isolation ohne Cross-Phase-Leak | ✅ §4.1, Gate 1 |
| `@include`-Fragment-Compose + **Registrierung** | ✅ §4.6, Gate 2/7 (🔴 Bugfix) |
| Body-Override D7 | ✅ §4.4, Gate 4 |
| Materialisierung A/B, Overlay `.lean-ctx/lean-md/` (D6/D8) | ✅ §3.3 |
| `skill install` in lean-md, lean-ctx-Installer entfernt | ✅ §4.7 + R3 |
| zero-config, Opt-in = Invocation (D10) | ✅ §4.7 |
| `availability`-Coverage als Gate | ✅ §4.8, Gate 8 (+ Companion-Zeile) |
| Determinismus #498, byte-stabile Seeds | ✅ Gate 7/9 |
| writing-skills SDO (description = nur Trigger) | ✅ §4.2, E10 |
| writing-skills „one excellent example", richtige Sprache | ✅ §4.1 Rust/nextest, code-frei (E13) |
| writing-skills Iron Law auf den Port angewandt | ✅ R2 (Port-Treue + Bootstrap) |

**Bewusste Erweiterungen über die Baseline:** (1) Skill-Registry-Generalisierung,
(2) `ctx_md_render` skill/phase/**companion**-Verdrahtung (E8), (3) `COVERAGE` skill-Dimension (E9),
(4) Naming + CRP-`tdd`-Kollisionsvermeidung (E2/E5), (5) **⟳ Fidelity-Restore der Disziplin-Tabellen** (E12),
(6) **⟳ Render-Bugfix als Pflicht-Gate** (E14).
