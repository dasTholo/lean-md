# Companion-Split + Dispatch-Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Den großen `testing-skills-with-subagents`-Companion in drei on-demand abrufbare `testing/*`-Einheiten splitten und die `@dispatch`-Bridge um eine `skill=`+`companion=`-Brief-Quelle plus eine `test`-Rolle erweitern, sodass die green-Phase einen Tester-Subagenten mit der Test-Methodik dispatchen kann.

**Architecture:** Zwei unabhängige, zusammen ausgelieferte Änderungen. (1) **Companion-Split** = reine Seed- + Registry- + Stub-Arbeit ohne Engine-Change: `companion_body`/`render_companion` matchen den Key als freien String, `"testing/methodology"` ist nur ein Key, Seeds bleiben `include_str!`-embedded (byte-stabil #498). (2) **Dispatch-Integration** = Engine-Change begrenzt auf `src/bridges/dispatch.rs`: zweite Brief-Quelle (`skill=`+`companion=` → `crate::skills::companion_body`) + Rolle `test`; das Skill wird explizit in der Direktive mitgegeben (kein `engine.rs`-Eingriff).

**Tech Stack:** Rust (standalone crate `lean_md`, lib + bin), `cargo nextest`, lean-ctx MCP-Tooling. Keine neuen Dependencies.

## Global Constraints

- **Tests:** immer `cargo nextest run`, nie `cargo test`. Crate ist standalone; Kommandos laufen aus dem Repo-Root (`/home/tholo/Scripts/lean-md`) — kein `cd`, kein `--manifest-path` nötig (Repo-Root ist CWD).
- **Shell:** kein `&&`/`||`/`;`-Chaining — jedes Kommando eine eigene Invocation.
- **Vor jedem `git add`** (je geänderte Code-Datei): `cargo fmt`.
- **No worktrees** — direkt auf `feat-lmd-v2`.
- **Determinismus (#498):** Tool-Output ist deterministische Funktion von (Inhalt, Mode, CRP, Task); embedded Seeds byte-identisch zur on-disk-Quelle (Fragment-/Companion-Consistency-Gates müssen grün bleiben); `CliBackend` == `McpBackend`. Keine Timestamps/Counter/Random.
- **Fidelity (kein Verlust):** die Summe der 3 neuen `testing/*`-Companions ist inhaltlich identisch zum bisherigen `testing-skills-with-subagents` — **verbatim**; nur die neuen Header-Kommentarzeilen unterscheiden sich. Keine Original-Sektion geht verloren.
- **Reference-Closure:** alle Querverweise zeigen auf lmd-/lean-md-Ziele, nie zurück nach superpowers.
- **Naming (E2):** Acronym „TDD" nur disambiguiert („TDD (test-driven development)"), nie nackt als Keyword. Bestehende Disambiguierungen bleiben.
- **Code/Kommentare:** Englisch. Interaktion/Commits-Prosa: Deutsch mit Umlauten (ä ö ü ß — nie ae/oe/ue/ss).
- **Tool-Discipline:** lean-ctx MCP-Tools statt nativ; `.lmd.md`-Rohquelle nur über `git show HEAD:<path>` lesbar (Shadow-Hook rendert sonst).

---

## File Structure

**Neue Seed-Dateien (alle embedded via `include_str!`):**

- `content/skills/lmd-writing-skills/companions/testing/methodology.lmd.md`
- `content/skills/lmd-writing-skills/companions/testing/skill-types.lmd.md`
- `content/skills/lmd-writing-skills/companions/testing/creation-checklist.lmd.md`

**Entfernte Seed-Datei:**

- `content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md`

**Geänderte Dateien:**

- `src/skills.rs` — Consts `LMD_WS_TESTING_SUBAGENTS` → drei (`LMD_WS_TESTING_METHODOLOGY`, `LMD_WS_TESTING_SKILL_TYPES`, `LMD_WS_TESTING_CREATION_CHECKLIST`); `COMPANIONS`-Zeile 1 → 3; Companion-Tests anpassen + neuer Fidelity-Test + green-phase Body-Test.
- `content/skills/lmd-writing-skills/SKILL.md` — Companion-Liste: 1 → 3 Einträge; Render-Call-Beispiel.
- `src/availability.rs` — `COVERAGE`: `testing-skills-with-subagents` → `testing/methodology`; neue `dispatch`-Zeile.
- `content/tooling/availability-audit.md` — writing-skills-Coverage-Abschnitt aktualisieren.
- `src/bridges/dispatch.rs` — `companion=`-Brief-Quelle + `test`-Rolle + Tests.
- `content/skills/lmd-writing-skills/body.lmd.md` — green-Phase erhält `@dispatch … companion="testing/methodology" role=test …`; refactor-Phase Re-Dispatch-Hinweis (nicht materialisiert).
- `src/bin/lean_md.rs` — CLI==MCP-Companion-Gate auf `testing/methodology`.

### Verbatim-Content-Hinweis (für ALLE Seed-Cuts)

Der Companion-Split verschiebt **verbatim** Content aus der Altdatei. Reproduktion von ~390 Zeilen im Plan wäre fehleranfällig — stattdessen extrahiert der Implementer die exakten Byte-Ranges aus der Altdatei und stellt nur die neue Header-Zeile voran. Die `.lmd.md`-Rohquelle ist NUR via `git show HEAD:<path>` lesbar (der Shadow-Hook rendert `Read`/`ctx_read` sonst). Schnittmarker (H2-Headings) sind exakt benannt — der Implementer schneidet an diesen Grenzen.

---

## Task 1: Companion-Split (Seeds + Registry + Stub + Tests)

Splittet `testing-skills-with-subagents.lmd.md` in drei `testing/*`-Seeds, registriert sie in `src/skills.rs`, entfernt die Altdatei, aktualisiert die SKILL.md-Companion-Liste und stellt die betroffenen Companion-Tests um. Deliverable: alle 10 writing-skills-Companions resolven, Fidelity grün, Altdatei weg.

**Files:**
- Create: `content/skills/lmd-writing-skills/companions/testing/methodology.lmd.md`
- Create: `content/skills/lmd-writing-skills/companions/testing/skill-types.lmd.md`
- Create: `content/skills/lmd-writing-skills/companions/testing/creation-checklist.lmd.md`
- Delete: `content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md`
- Modify: `src/skills.rs` (Consts ~Z.27-29, `COMPANIONS` ~Z.85-89, Tests)
- Modify: `content/skills/lmd-writing-skills/SKILL.md` (Companion-Liste)

**Interfaces:**
- Consumes: bestehende `companion_body(skill, companion) -> Option<&'static str>` und `render_companion(skill, companion, consumer, crp, jail_root) -> Result<String, SkillRenderError>` (unverändert — matchen den Key als freien String).
- Produces: drei neue Companion-Keys `"testing/methodology"`, `"testing/skill-types"`, `"testing/creation-checklist"` unter Skill `"lmd-writing-skills"`. `"testing-skills-with-subagents"` existiert danach NICHT mehr. Task 3 (`@dispatch`) und Task 4 (body) konsumieren `"testing/methodology"`.

**Schnittgrenzen (Fidelity-Coverage-Matrix):**

| Neue Datei | Quell-Bereich (alte Datei) | Neue Header-Kommentarzeile (Zeile 1) | `@include`? |
|---|---|---|---|
| `testing/methodology.lmd.md` | Header-`@include` + `# Testing Skills With Subagents` … bis Ende `## Real-World Impact` (letzte Zeile: `- Same process works for any discipline-enforcing skill`) | `# Testing Methodology (lmd companion — RED→GREEN→REFACTOR testing workflow for skills)` | **ja** (`@include skill-authoring-core` bleibt am Kopf) |
| `testing/skill-types.lmd.md` | `## Testing All Skill Types` … bis Ende `### Reference Skills` (letzte Zeile: `**Success criteria:** Agent finds and correctly applies reference information`) | `# Testing Skill Types (lmd companion — how to test discipline/technique/pattern/reference skills)` | **nein** |
| `testing/creation-checklist.lmd.md` | `## Skill Creation Checklist (TDD Adapted)` … bis EOF (letzte Zeile: `- [ ] Consider contributing back via PR (if broadly useful)`) | `# Skill Creation Checklist (lmd companion — TDD-adapted checklist before deploying a skill)` | **nein** |

**Regeln:**
- Nur `methodology` behält `@include skill-authoring-core` (Disziplin-Erzählung → Iron-Law-Marker `NO SKILL WITHOUT A FAILING TEST FIRST`). `skill-types` + `creation-checklist` sind reine Referenz/Checkliste → **kein** `@include`.
- Jede Datei: Header-Zeile, dann Leerzeile, dann verbatim der Quell-Bereich.
- **Reference-Closure:** Zwischen den 3 neuen Companions existieren KEINE Querverweise (Original waren angehängte Sektionen einer Datei) → keine `ctx_md_render`-Closure-Edits nötig. Die bestehenden geschlossenen Verweise in `methodology` (`companion="claude-md-testing-example"`, `companion="persuasion-principles"`) bleiben unverändert verbatim.

- [ ] **Step 1: Schnittgrenzen verifizieren (Altdatei-Headings auflisten)**

Run:
```bash
git show HEAD:content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md > /tmp/claude-1000/-home-tholo-Scripts-lean-md/acba9045-ffe1-42f6-9a42-abb3b9e3ac04/scratchpad/old-testing.md
```
Dann die H2-Marker prüfen (über `ctx_search` oder `ctx_read` der Kopie). Expected: `## Real-World Impact` steht direkt vor `## Testing All Skill Types`; `## Skill Creation Checklist (TDD Adapted)` ist die letzte H2 vor EOF.

- [ ] **Step 2: `testing/methodology.lmd.md` schreiben**

Header-Zeile 1 = `# Testing Methodology (lmd companion — RED→GREEN→REFACTOR testing workflow for skills)`, Leerzeile, dann verbatim ab `@include skill-authoring-core` (alte Zeile 3) bis inkl. der `## Real-World Impact`-Sektion. Die alte Zeile-1-Header (`# Testing Skills With Subagents (lmd companion — full testing methodology …)`) wird NICHT übernommen; die alte H1 `# Testing Skills With Subagents` (alte Zeile 5) bleibt erhalten.

- [ ] **Step 3: `testing/skill-types.lmd.md` schreiben**

Header-Zeile 1 = `# Testing Skill Types (lmd companion — how to test discipline/technique/pattern/reference skills)`, Leerzeile, dann verbatim ab `## Testing All Skill Types` bis Ende `### Reference Skills`. **Kein** `@include`.

- [ ] **Step 4: `testing/creation-checklist.lmd.md` schreiben**

Header-Zeile 1 = `# Skill Creation Checklist (lmd companion — TDD-adapted checklist before deploying a skill)`, Leerzeile, dann verbatim ab `## Skill Creation Checklist (TDD Adapted)` bis EOF. **Kein** `@include`.

- [ ] **Step 5: Altdatei entfernen**

Run:
```bash
git rm content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md
```

- [ ] **Step 6: Consts in `src/skills.rs` umstellen**

Ersetze die `LMD_WS_TESTING_SUBAGENTS`-Const-Deklaration durch drei:

```rust
const LMD_WS_TESTING_METHODOLOGY: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/testing/methodology.lmd.md"
);
const LMD_WS_TESTING_SKILL_TYPES: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/testing/skill-types.lmd.md"
);
const LMD_WS_TESTING_CREATION_CHECKLIST: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/testing/creation-checklist.lmd.md"
);
```

- [ ] **Step 7: `COMPANIONS`-Registry in `src/skills.rs` umstellen**

Ersetze den Tuple-Eintrag

```rust
    (
        "lmd-writing-skills",
        "testing-skills-with-subagents",
        LMD_WS_TESTING_SUBAGENTS,
    ),
```

durch:

```rust
    (
        "lmd-writing-skills",
        "testing/methodology",
        LMD_WS_TESTING_METHODOLOGY,
    ),
    (
        "lmd-writing-skills",
        "testing/skill-types",
        LMD_WS_TESTING_SKILL_TYPES,
    ),
    (
        "lmd-writing-skills",
        "testing/creation-checklist",
        LMD_WS_TESTING_CREATION_CHECKLIST,
    ),
```

- [ ] **Step 8: Companion-Tests in `src/skills.rs` umstellen**

In `writing_skills_all_companions_resolve` die `names`-Liste umstellen (8 → 10 Einträge):

```rust
        let names = [
            "skill-anatomy",
            "skill-discovery-optimization",
            "bulletproofing",
            "testing/methodology",
            "testing/skill-types",
            "testing/creation-checklist",
            "claude-md-testing-example",
            "flowchart-conventions",
            "anthropic-best-practices",
            "persuasion-principles",
        ];
```

In `writing_skills_discipline_companions_carry_trip_wire` die Loop-Liste umstellen (nur `methodology` trägt das Trip-Wire):

```rust
        for n in ["testing/methodology", "bulletproofing"] {
```

`writing_skills_testing_companion_carries_skill_md_sections` umstellen — Sektionen liegen jetzt in den getrennten Companions:

```rust
    #[test]
    fn writing_skills_testing_companion_carries_skill_md_sections() {
        let jail = std::path::PathBuf::from(".");
        let types =
            render_companion("lmd-writing-skills", "testing/skill-types", None, None, jail.clone())
                .unwrap();
        assert!(
            types.contains("Testing All Skill Types"),
            "skill-types companion must carry the 'Testing All Skill Types' section (fidelity)"
        );
        let checklist = render_companion(
            "lmd-writing-skills",
            "testing/creation-checklist",
            None,
            None,
            jail,
        )
        .unwrap();
        assert!(
            checklist.contains("Skill Creation Checklist (TDD Adapted)"),
            "creation-checklist companion must carry the 'Skill Creation Checklist' section (fidelity)"
        );
    }
```

In `writing_skills_fidelity_all_surfaces_render_nonempty` die Companion-Liste umstellen (gleiche 10 Namen wie Step 8 oben):

```rust
        for c in [
            "skill-anatomy",
            "skill-discovery-optimization",
            "bulletproofing",
            "testing/methodology",
            "testing/skill-types",
            "testing/creation-checklist",
            "claude-md-testing-example",
            "flowchart-conventions",
            "anthropic-best-practices",
            "persuasion-principles",
        ] {
```

- [ ] **Step 9: Neuen Fidelity-Test in `src/skills.rs` hinzufügen**

```rust
    #[test]
    fn writing_skills_testing_split_carries_all_original_sections() {
        let jail = std::path::PathBuf::from(".");
        let methodology =
            render_companion("lmd-writing-skills", "testing/methodology", None, None, jail.clone())
                .unwrap();
        // Methodology marker + Iron Law via @include skill-authoring-core.
        assert!(
            methodology.contains("RED Phase: Baseline Testing"),
            "methodology must carry the RED-baseline section: {methodology}"
        );
        assert!(
            methodology.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "methodology must @include skill-authoring-core (Iron Law)"
        );
        let types =
            render_companion("lmd-writing-skills", "testing/skill-types", None, None, jail.clone())
                .unwrap();
        assert!(types.contains("Reference Skills"), "skill-types fidelity");
        let checklist = render_companion(
            "lmd-writing-skills",
            "testing/creation-checklist",
            None,
            None,
            jail,
        )
        .unwrap();
        assert!(
            checklist.contains("Deployment"),
            "creation-checklist fidelity (Deployment section)"
        );
    }
```

- [ ] **Step 10: SKILL.md-Companion-Liste umstellen**

In `content/skills/lmd-writing-skills/SKILL.md` den Eintrag

```
- `testing-skills-with-subagents` — full testing methodology + creation checklist
```

ersetzen durch:

```
- `testing/methodology` — RED→GREEN→REFACTOR testing workflow (pressure scenarios, rationalization tables)
- `testing/skill-types` — how to test discipline/technique/pattern/reference skills
- `testing/creation-checklist` — TDD-adapted checklist before deploying a skill
```

(Die `ctx_md_render(skill="lmd-writing-skills", companion="<name>")`-Beispielzeile bleibt unverändert — `<name>` ist generisch.)

- [ ] **Step 11: fmt + Tests**

Run:
```bash
cargo fmt
```
Run:
```bash
cargo nextest run skills::
```
Expected: PASS — `writing_skills_all_companions_resolve`, `writing_skills_discipline_companions_carry_trip_wire`, `writing_skills_testing_companion_carries_skill_md_sections`, `writing_skills_fidelity_all_surfaces_render_nonempty`, `writing_skills_testing_split_carries_all_original_sections` alle grün; kein Test referenziert mehr `testing-skills-with-subagents`.

- [ ] **Step 12: Commit**

```bash
git add content/skills/lmd-writing-skills/companions/testing src/skills.rs content/skills/lmd-writing-skills/SKILL.md
git add -A content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md
git commit -m "feat(lmd-ws): split testing companion into testing/{methodology,skill-types,creation-checklist}"
```

---

## Task 2: COVERAGE + Audit-Doc

Aktualisiert `src/availability.rs::COVERAGE` (Companion-Zeile umbenennen + Dispatch-Zeile ergänzen) und die menschenlesbare Projektion in `content/tooling/availability-audit.md`. Deliverable: `every_covered_directive_is_registered` grün, Audit-Doc spiegelt den Split.

**Files:**
- Modify: `src/availability.rs` (`COVERAGE`-Array, writing-skills-Block)
- Modify: `content/tooling/availability-audit.md` (writing-skills-Coverage-Abschnitt)

**Interfaces:**
- Consumes: Companion-Key `"testing/methodology"` aus Task 1; die registrierte Direktive `"dispatch"` (bereits in `default_registry()` — bestätigt durch `dispatch_is_registered`).
- Produces: keine API; nur Coverage-Daten. Das Gate `every_covered_directive_is_registered` prüft, dass `"include"` und `"dispatch"` registriert sind.

- [ ] **Step 1: `COVERAGE`-Array in `src/availability.rs` umstellen**

Ersetze den writing-skills-Companion-Eintrag

```rust
    (
        "lmd-writing-skills",
        "testing-skills-with-subagents",
        "include",
        "fragment-compose",
    ),
```

durch zwei Zeilen:

```rust
    (
        "lmd-writing-skills",
        "testing/methodology",
        "include",
        "fragment-compose",
    ),
    // green-Phase dispatcht den Tester-Subagenten (Brief = testing/methodology).
    (
        "lmd-writing-skills",
        "green",
        "dispatch",
        "fragment-compose",
    ),
```

- [ ] **Step 2: Gate ausführen**

Run:
```bash
cargo nextest run availability::
```
Expected: PASS — `every_covered_directive_is_registered` grün (`dispatch` + `include` sind registriert), `coverage_carries_skill_dimension` + `coverage_carries_companion_row` weiterhin grün (prüfen die TDD-Companion-Zeile, unverändert).

- [ ] **Step 3: Audit-Doc aktualisieren**

In `content/tooling/availability-audit.md` den `## lmd-writing-skills — Coverage`-Block ersetzen:

```markdown
## lmd-writing-skills — Coverage

| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing |
| red (baseline read) | `@read` | `ctx_read` |
| green (tester dispatch) | `@dispatch` | fragment-compose |
| companion (@include skill-authoring-core) | `@include` | fragment-compose |

Die green-Phase dispatcht einen Tester-Subagenten, dessen Brief der Companion
`testing/methodology` ist (`@dispatch skill="lmd-writing-skills"
companion="testing/methodology" role=test`). Test execution (subagent pressure
scenarios) bleibt Prosa-Disziplin, keine registrierte Direktive — transparent
hier vermerkt.
```

- [ ] **Step 4: fmt + Commit**

```bash
cargo fmt
git add src/availability.rs content/tooling/availability-audit.md
git commit -m "feat(lmd-ws): COVERAGE testing/methodology + green dispatch row; audit doc"
```

---

## Task 3: Dispatch-Engine — `companion=`-Brief-Quelle + `test`-Rolle

Erweitert `src/bridges/dispatch.rs` um eine zweite Brief-Quelle (`skill=`+`companion=` → `companion_body`) exklusiv zu `phase=`, plus die Rolle `test`. Alle bestehenden Guards (M-2 to_agent-Injection, `{{ controller_id }}`-Erhalt, M-3 NUL-frei, CRP) bleiben. Deliverable: beide Brief-Pfade grün, exklusive Validierung, `COMPANION_NOT_FOUND`-Envelope.

**Files:**
- Modify: `src/bridges/dispatch.rs` (`execute` Brief-Source-Block + Role-Match + Tests)

**Interfaces:**
- Consumes: `crate::skills::companion_body(skill, companion) -> Option<&'static str>` (aus Task 1); `crate::render::splice_template_only(ctx, &str) -> String` (work-lazy Render); `DirectiveArgs::get(&str) -> Option<&str>`, `DirectiveArgs::positional(usize)`; `BridgeError::{MissingArg(&'static str), Resolve(String)}`.
- Produces: `@dispatch` akzeptiert jetzt `skill=` + `companion=` als Brief-Quelle (XOR mit `phase=`) und Rolle `test`. Task 4 (body) nutzt `@dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test`.

**Verhalten (Spec Detail 3):**
- Brief-Quelle: genau eine von `phase=<name>` ODER `skill=<skill>`+`companion=<companion>`.
- Beide gegeben → `BridgeError::Resolve("use exactly one of phase= or companion=")`.
- Keine gegeben → `BridgeError::MissingArg("phase")` (wie bisher).
- `companion=` ohne `skill=` → `BridgeError::MissingArg("skill")`.
- Companion nicht gefunden → `<!-- lmd: COMPANION_NOT_FOUND '<skill>/<companion>' -->\n` Envelope (kein Abbruch).
- Companion-Body wird **work-lazy** via `splice_template_only` gerendert (Templates/`{{ }}` + `@include skill-authoring-core` aufgelöst, Work-Direktiven wie `@read` verbatim). Komposition unverändert: Contract (b) + Brief unter `## Task (phase-isolated)` + Bootstrap (c).
- Rolle `test`: Validierung `Some(r @ ("dev" | "review" | "test"))`; `{{ role }}` → `test`.

- [ ] **Step 1: Failing Tests schreiben (`src/bridges/dispatch.rs` `mod tests`)**

```rust
    #[test]
    fn dispatch_companion_brief_composes_contract_methodology_bootstrap() {
        let doc = "@dispatch skill=\"lmd-writing-skills\" companion=\"testing/methodology\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(out.contains("role=test"), "test role missing: {out}");
        assert!(out.contains("RED Phase"), "methodology marker missing: {out}");
        assert!(
            out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "Iron Law via @include missing: {out}"
        );
        assert!(
            out.contains("ToolSearch(query=\"select:mcp__lean-ctx__ctx_read"),
            "bootstrap missing: {out}"
        );
    }

    #[test]
    fn dispatch_phase_source_still_works() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("role=dev"), "phase path regressed: {out}");
        assert!(out.contains("@read a.rs"), "work directive verbatim: {out}");
    }

    #[test]
    fn dispatch_rejects_both_phase_and_companion() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" skill=\"lmd-writing-skills\" companion=\"testing/methodology\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("exactly one of phase= or companion="),
            "both-given must Resolve-error: {out}"
        );
    }

    #[test]
    fn dispatch_companion_requires_skill() {
        let doc = "@dispatch companion=\"testing/methodology\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("skill") && (out.contains("MissingArg") || out.contains("missing")),
            "companion= without skill= must MissingArg(skill): {out}"
        );
    }

    #[test]
    fn dispatch_unknown_companion_yields_envelope() {
        let doc = "@dispatch skill=\"lmd-writing-skills\" companion=\"nope\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("COMPANION_NOT_FOUND 'lmd-writing-skills/nope'"),
            "unknown companion must yield envelope, not abort: {out}"
        );
    }

    #[test]
    fn dispatch_test_role_substitutes() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("role=test"), "test role must substitute: {out}");
    }
```

> **Hinweis zum Test-Harness:** `render(doc)` (Helper `crate::engine::render`) baut den `EngineContext` mit Default-Jail; Fragmente (`skill-authoring-core`, `dispatch-contract`) resolven embedded — derselbe Pfad wie die bestehenden Companion-Tests in `src/skills.rs`, die mit Jail `.` grün sind. `MissingArg`/`Resolve` werden vom Render-Pfad als sichtbarer Fehlertext in den Output gespiegelt (vgl. bestehender `invalid_role_is_rejected`-Test, der `render` nutzt und auf `"unknown @dispatch role"` prüft).

- [ ] **Step 2: Tests ausführen — müssen fehlschlagen**

Run:
```bash
cargo nextest run dispatch::
```
Expected: FAIL — die 6 neuen Tests scheitern (`companion=` noch nicht implementiert; `role=test` noch abgelehnt). `dispatch_phase_source_still_works` sollte bereits grün sein.

- [ ] **Step 3: Brief-Source-Block in `execute` ersetzen**

Ersetze in `src/bridges/dispatch.rs::execute` den bisherigen Block

```rust
        let phase = args
            .get("phase")
            .or_else(|| args.positional(0))
            .ok_or(BridgeError::MissingArg("phase"))?;

        // (a) phasen-isolierter Body — Lookup im capture-Pre-Pass (C1).
        let Some(raw_body) = ctx.phase_body(phase) else {
            return Ok(format!("<!-- lmd: PHASE_NOT_FOUND '{phase}' -->\n"));
        };
```

durch die exklusive Zwei-Quellen-Auflösung (`raw_body` als `Cow`, da `phase_body` owned + `companion_body` borrowed liefert):

```rust
        // Brief source (Spec Detail 3): exactly one of phase= OR skill=+companion=.
        let phase = args.get("phase").or_else(|| args.positional(0));
        let companion = args.get("companion");
        let raw_body: std::borrow::Cow<'_, str> = match (phase, companion) {
            (Some(_), Some(_)) => {
                return Err(BridgeError::Resolve(
                    "use exactly one of phase= or companion=".to_string(),
                ));
            }
            (Some(p), None) => {
                // (a) phasen-isolierter Body — Lookup im capture-Pre-Pass (C1).
                let Some(body) = ctx.phase_body(p) else {
                    return Ok(format!("<!-- lmd: PHASE_NOT_FOUND '{p}' -->\n"));
                };
                std::borrow::Cow::Owned(body)
            }
            (None, Some(c)) => {
                let skill = args.get("skill").ok_or(BridgeError::MissingArg("skill"))?;
                // (a') companion brief — embedded source, rendered work-lazy below.
                let Some(body) = crate::skills::companion_body(skill, c) else {
                    return Ok(format!("<!-- lmd: COMPANION_NOT_FOUND '{skill}/{c}' -->\n"));
                };
                std::borrow::Cow::Borrowed(body)
            }
            (None, None) => return Err(BridgeError::MissingArg("phase")),
        };
```

- [ ] **Step 4: Role-Match um `test` erweitern**

Ersetze den Role-Match:

```rust
        let role = match args.get("role") {
            Some(r @ ("dev" | "review")) => r,
            Some(other) => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @dispatch role '{other}'. Use: dev|review"
                )));
            }
            None => "dev",
        };
```

durch:

```rust
        let role = match args.get("role") {
            Some(r @ ("dev" | "review" | "test")) => r,
            Some(other) => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @dispatch role '{other}'. Use: dev|review|test"
                )));
            }
            None => "dev",
        };
```

- [ ] **Step 5: `splice_template_only`-Call an `Cow` anpassen**

Der bestehende Body-Render-Aufruf nutzt `&raw_body`. Mit `raw_body: Cow<str>` dereft `&raw_body` weiterhin zu `&str` (Cow: Deref<Target=str>) — der Aufruf

```rust
        let body_rendered = crate::render::splice_template_only(ctx, &raw_body);
```

bleibt unverändert gültig. Verifiziere, dass keine weitere Stelle `phase` als `&str` annimmt (der frühere `phase`-Binding ist jetzt `Option<&str>`; er wird nur noch im Match konsumiert).

- [ ] **Step 6: Tests ausführen — müssen bestehen**

Run:
```bash
cargo nextest run dispatch::
```
Expected: PASS — alle 6 neuen Tests grün; bestehende (`composes_contract_body_and_bootstrap_with_work_lazy`, `review_role_substitutes`, `missing_to_agent_warns_but_does_not_abort`, `invalid_role_is_rejected`, `to_agent_template_injection_is_neutralized`, `rendered_output_contains_no_nul_bytes`, CRP-Tests) bleiben grün.

- [ ] **Step 7: fmt + Commit**

```bash
cargo fmt
git add src/bridges/dispatch.rs
git commit -m "feat(dispatch): companion= brief source (XOR phase=) + test role"
```

---

## Task 4: Body-Verdrahtung + CLI==MCP-Sample

Verdrahtet die green-Phase mit dem materialisierten Tester-`@dispatch` und gibt der refactor-Phase einen nicht-materialisierten Re-Dispatch-Hinweis. Fügt den green-Phasen-Render-Test plus ein CLI==MCP-Byte-Stabilitäts-Sample für die `testing/methodology`-Surface hinzu. Deliverable: green rendert den vollen Tester-Prompt, refactor leakt nicht, Determinismus-Sample grün.

**Files:**
- Modify: `content/skills/lmd-writing-skills/body.lmd.md` (green + refactor Phase)
- Modify: `src/skills.rs` (`mod tests` — neuer green-Body-Test)
- Modify: `src/bin/lean_md.rs` (`mod tests` — CLI==MCP-Gate auf `testing/methodology`)

**Interfaces:**
- Consumes: `@dispatch` mit `skill=`+`companion=`+`role=test` (aus Task 3); Companion `testing/methodology` (aus Task 1); `render_skill("lmd-writing-skills", Some("green"), …)`; `render_companion("lmd-writing-skills", "testing/methodology", …)`.
- Produces: keine neue API — finale Verdrahtung der Surface.

- [ ] **Step 1: green-Phase Body-Test schreiben (`src/skills.rs` `mod tests`)**

```rust
    #[test]
    fn green_phase_renders_tester_dispatch_block() {
        let out = render_skill(
            "lmd-writing-skills",
            Some("green"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        // @dispatch materialised: contract + methodology marker + Iron Law + bootstrap.
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(out.contains("RED Phase"), "methodology brief missing: {out}");
        assert!(
            out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "Iron Law via @include missing: {out}"
        );
        assert!(out.contains("role=test"), "test role missing: {out}");
        // to_agent placeholder kept fillable (M-2 guard injects it literally).
        assert!(
            out.contains("to_agent={{ controller_id }}"),
            "controller_id placeholder must survive verbatim: {out}"
        );
        // Phase isolation: refactor's re-dispatch hint must NOT leak into green.
        assert!(
            !out.contains("re-dispatch the same tester"),
            "refactor content leaked into green: {out}"
        );
    }
```

- [ ] **Step 2: Test ausführen — muss fehlschlagen**

Run:
```bash
cargo nextest run skills::green_phase_renders_tester_dispatch_block
```
Expected: FAIL — die green-Phase trägt den Dispatch-Block noch nicht.

- [ ] **Step 3: green-Phase in `body.lmd.md` erweitern**

Nach dem bestehenden GREEN-Block (direkt vor `next: render phase "refactor".`) den Dispatch-Block einfügen. Die green-Phase lautet danach (Änderung = die zwei Absätze vor der `next:`-Zeile):

```
@phase "green"
@include skill-authoring-core

## GREEN — write the minimal skill

Write the skill that addresses those SPECIFIC baseline rationalizations — nothing
for hypothetical cases. Then run the same scenarios WITH the skill: the agent must
now comply.

Match the form to the failure:
- skips/violates a rule under pressure -> prohibition + rationalization table + red flags
- output has the wrong shape -> a positive recipe/contract stating what the output IS
- omits a required element -> a REQUIRED structural slot in the template they fill in
- behavior should depend on a condition -> a conditional keyed to an observable predicate

Micro-test the wording before full scenarios: one fresh-context sample per call,
always a no-guidance control, 5+ reps, read every flagged match manually, treat
variance as a metric (five interpretations across five reps = wording not binding).

To pressure-test the skill you just wrote, dispatch a tester subagent whose brief
is the full testing methodology:

@dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test to_agent="{{ controller_id }}"

next: render phase "refactor".
@phase-end
```

> **Warum `{{ controller_id }}` literal überlebt:** die M-2-Guard in `dispatch.rs` ersetzt `{{ controller_id }}` im Contract durch ein Sentinel VOR `render_body` und injiziert den `to_agent`-Wert (hier den Literal-String `{{ controller_id }}`) DANACH — er erscheint als `to_agent={{ controller_id }}` (analog zum bestehenden `to_agent_template_injection_is_neutralized`-Test).

- [ ] **Step 4: refactor-Phase Re-Dispatch-Hinweis (nicht materialisiert)**

In der refactor-Phase einen Prosa-Hinweis ergänzen. Die `@dispatch`-Direktive MUSS in Inline-Code-Backticks stehen, damit sie beim Rendern NICHT materialisiert (vermeidet doppelte große Dispatch-Blöcke; green trägt die kanonische ausführbare Direktive). Füge nach dem bestehenden `ctx_md_render(... companion="bulletproofing")`-Absatz hinzu:

```
After closing loopholes, re-dispatch the same tester
(`@dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test`)
and re-verify the agent still complies under pressure.
```

- [ ] **Step 5: green-Body-Test ausführen — muss bestehen**

Run:
```bash
cargo nextest run skills::green_phase_renders_tester_dispatch_block
```
Expected: PASS.

- [ ] **Step 6: Phasen-Isolation gegenprüfen (refactor leakt nicht in green; bestehende Iso-Tests)**

Run:
```bash
cargo nextest run skills::writing_skills_phases_are_isolated
```
Expected: PASS — green trägt weiterhin das Trip-Wire und `write the minimal skill`, nicht den refactor-spezifischen Re-Dispatch-Satz.

- [ ] **Step 7: CLI==MCP-Gate in `src/bin/lean_md.rs` auf `testing/methodology` umstellen**

Ersetze in `ws_mcp_companion_matches_cli_render_companion` den Companion-Namen `"skill-anatomy"` durch `"testing/methodology"` und füge eine Marker-Assertion hinzu:

```rust
    #[test]
    fn ws_mcp_companion_matches_cli_render_companion() {
        // CLI==MCP (#498): both surfaces call render_companion → byte-identical.
        let jail = std::path::PathBuf::from(".");
        let cli = render_companion(
            "lmd-writing-skills",
            "testing/methodology",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        let again = render_companion(
            "lmd-writing-skills",
            "testing/methodology",
            None,
            None,
            jail,
        )
        .unwrap();
        assert_eq!(
            cli, again,
            "render_companion must be a deterministic function (#498)"
        );
        assert!(
            cli.contains("RED Phase"),
            "testing/methodology surface must render its methodology body"
        );
    }
```

- [ ] **Step 8: Gate ausführen**

Run:
```bash
cargo nextest run --bin lean-md
```
Expected: PASS — `ws_mcp_companion_matches_cli_render_companion` byte-stabil auf der neuen Surface; `mcp_companion_matches_cli_render_companion` (TDD-Companion) unverändert grün.

- [ ] **Step 9: fmt + Commit**

```bash
cargo fmt
git add content/skills/lmd-writing-skills/body.lmd.md src/skills.rs src/bin/lean_md.rs
git commit -m "feat(lmd-ws): green-phase dispatches tester (testing/methodology); refactor re-dispatch hint; CLI==MCP sample"
```

---

## Task 5: Full-Gate (finaler Task — ersetzt Port-Plan Task 8)

Führt den vollständigen Determinismus-/Qualitäts-Gate über das gesamte Crate aus. Ersetzt das offene Task 8 (Full-Gate) des Port-Plans `2026-06-29-lmd-writing-skills-port`. Deliverable: fmt-check, komplette Suite, clippy `-D warnings`, Render-Smoke aller neuen Surfaces grün.

**Files:** keine (reiner Verifikations-Task).

- [ ] **Step 1: Format-Check (kein Diff)**

Run:
```bash
cargo fmt --check
```
Expected: kein Output, Exit 0. (Bei Diff: `cargo fmt`, betroffene Datei committen, erneut prüfen.)

- [ ] **Step 2: Clippy mit `-D warnings`**

Run:
```bash
cargo clippy --all-targets -- -D warnings
```
Expected: `Finished` ohne Warnungen/Fehler.

- [ ] **Step 3: Komplette Test-Suite**

Run:
```bash
cargo nextest run
```
Expected: alle Tests PASS — insbesondere `skills::*` (Companion-Resolution + Fidelity + green-Dispatch), `dispatch::*` (beide Brief-Pfade + test-Rolle + Guards), `availability::*` (Coverage-Gate), `lean-md`-Bin (CLI==MCP).

- [ ] **Step 4: Render-Smoke aller neuen Surfaces (CLI)**

Run:
```bash
cargo run --bin lean-md -- render --skill lmd-writing-skills --companion testing/methodology
```
Expected: rendert die Methodik inkl. `NO SKILL WITHOUT A FAILING TEST FIRST` + `RED Phase: Baseline Testing`.

Run:
```bash
cargo run --bin lean-md -- render --skill lmd-writing-skills --companion testing/skill-types
```
Expected: enthält `Testing All Skill Types`.

Run:
```bash
cargo run --bin lean-md -- render --skill lmd-writing-skills --companion testing/creation-checklist
```
Expected: enthält `Skill Creation Checklist (TDD Adapted)`.

Run:
```bash
cargo run --bin lean-md -- render --skill lmd-writing-skills --phase green
```
Expected: enthält den materialisierten Dispatch-Block (`Subagent Contract`, `role=test`, `RED Phase`, `to_agent={{ controller_id }}`).

- [ ] **Step 5: Determinismus-Stichprobe (CLI==MCP byte-stabil)**

Run:
```bash
cargo run --bin lean-md -- render --skill lmd-writing-skills --companion testing/methodology -o /tmp/claude-1000/-home-tholo-Scripts-lean-md/acba9045-ffe1-42f6-9a42-abb3b9e3ac04/scratchpad/m1.md
```
Run:
```bash
cargo run --bin lean-md -- render --skill lmd-writing-skills --companion testing/methodology -o /tmp/claude-1000/-home-tholo-Scripts-lean-md/acba9045-ffe1-42f6-9a42-abb3b9e3ac04/scratchpad/m2.md
```
Run:
```bash
diff /tmp/claude-1000/-home-tholo-Scripts-lean-md/acba9045-ffe1-42f6-9a42-abb3b9e3ac04/scratchpad/m1.md /tmp/claude-1000/-home-tholo-Scripts-lean-md/acba9045-ffe1-42f6-9a42-abb3b9e3ac04/scratchpad/m2.md
```
Expected: kein Output (byte-identisch, #498).

- [ ] **Step 6: Abschluss-Commit (falls fmt/clippy etwas berührt hat)**

Nur falls Steps 1-2 Änderungen erzwangen:
```bash
git add -A
git commit -m "chore(lmd-ws): full-gate green — fmt, clippy -D warnings, suite, render smoke"
```
Sonst: kein Commit nötig — der Gate ist reine Verifikation über die Task-1-4-Commits.

---

## Self-Review

**1. Spec-Coverage:**
- Companion-Split (Seeds + Registry + Schnittgrenzen) → Task 1. ✓
- SKILL.md-Stub (1→3) → Task 1 Step 10. ✓
- COVERAGE-Rename + Dispatch-Zeile → Task 2. ✓
- Audit-Doc → Task 2 Step 3. ✓
- Dispatch-Engine (XOR-Brief, `test`-Rolle, COMPANION_NOT_FOUND, work-lazy) → Task 3. ✓
- Body-Verdrahtung (green materialisiert, refactor backticked) → Task 4 Steps 3-4. ✓
- CLI==MCP-Gate auf `testing/methodology` → Task 4 Step 7. ✓
- Alle Spec-Detail-5-Tests (Split, Dispatch, Body, Determinismus) → Tasks 1/3/4. ✓
- Full-Gate als finaler Task → Task 5. ✓

**2. Placeholder-Scan:** keine TBD/TODO; alle Schnittgrenzen mit exakten Marker-Zeilen, alle Rust-Blöcke vollständig, alle Kommandos mit Expected. Verbatim-Content-Cuts sind als Byte-Range-Extraktion (`git show`) spezifiziert statt 390 Zeilen zu duplizieren — bewusste, ehrliche Entscheidung (Content existiert verbatim; Plan liefert Grenzen + Deltas).

**3. Typ-Konsistenz:** Companion-Keys `"testing/methodology"`/`"testing/skill-types"`/`"testing/creation-checklist"` identisch in Seed-Pfaden, Consts, `COMPANIONS`, SKILL.md, COVERAGE, allen Tests, `@dispatch`-Direktive. Consts `LMD_WS_TESTING_{METHODOLOGY,SKILL_TYPES,CREATION_CHECKLIST}` konsistent zwischen Deklaration (Step 6) und Registry (Step 7). `companion_body`/`render_companion`/`splice_template_only`-Signaturen gegen die echten Quellen verifiziert. `raw_body: Cow<str>` löst die `phase_body`(owned)/`companion_body`(borrowed)-Typdivergenz korrekt; `&raw_body` dereft zu `&str` für `splice_template_only`.
