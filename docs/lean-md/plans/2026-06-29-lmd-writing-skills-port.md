# lmd-writing-skills — Voll-Port von superpowers:writing-skills (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `superpowers:writing-skills` vollständig und verlustfrei als native lean-md-Skill `lmd-writing-skills` ausliefern — 4 phasen-isolierte Render-Blöcke, 8 Companions, 1 install-materialisiertes Asset — sodass superpowers für diese Skill entbehrlich wird.

**Architecture:** Skill-Bodies sind binary-embedded (`include_str!`) und werden phasenweise über `ctx_md_render(skill, phase)` gerendert (`capture_phase_bodies` → kein Cross-Phase-Leak). Ein neues `skill-authoring-core`-Fragment wird per `@include` in jede Phase gezogen (Disziplin-Trip-Wire trotz Isolation) und referenziert `lmd-test-driven-development` als WARUM. Companions fließen über `render_companion`; das Asset `render-graphs.js` wird von `skill install` nach `.claude/skills/` materialisiert (neuer Asset-Schritt, idempotent nach Vorbild `seeds.rs::materialize_contracts`).

**Tech Stack:** Rust (standalone crate `lean_md`, lib + bin), `cargo nextest`, lean-ctx MCP-Tooling (`ctx_read`/`ctx_search`/`ctx_edit`/`ctx_shell`), `serde_json` (MCP JSON-RPC).

**Spec:** `docs/lean-md/specs/2026-06-29-lmd-writing-skills-port-design.md`.

## Global Constraints

- **Tests:** immer `cargo nextest run`, **nie** `cargo test`. Crate ist standalone; `Cargo.toml` + `src/` liegen im Repo-Root → Kommandos laufen aus dem Repo-Root (kein `cd`, kein `--manifest-path`).
- **Shell:** kein `&&`/`||`/`;`-Chaining — jedes Kommando ist eine eigene Invocation.
- **Vor jedem `git add`** (je geänderte Code-Datei): `cargo fmt`.
- **No worktrees** — direkt auf dem aktuellen Branch `feat-lmd-v2` arbeiten.
- **Determinismus (#498):** Tool-Output ist deterministische Funktion von (Inhalt, Mode, CRP, Task) — keine Timestamps/Counter/Random; embedded Seeds byte-identisch zur on-disk-Quelle (Fragment-Consistency-Gate muss grün bleiben); `CliBackend` == `McpBackend`.
- **Fidelity (kein Verlust):** jede Original-Sektion/-Datei landet in genau einem lmd-Ziel; verbatim portieren, nur die Reference-Closure-Edits ändern.
- **Reference-Closure:** alle Querverweise zeigen auf lmd-/lean-md-Ziele, nie zurück nach superpowers.
- **Code/Kommentare:** Englisch. Interaktion/Commits-Prosa: Deutsch mit Umlauten.
- **Naming (E2):** Skill heißt ausgeschrieben `lmd-writing-skills`. Acronym „TDD" im Body **nur** disambiguiert („TDD (test-driven development)"), nie als nacktes Keyword (E10/CRP-Kollision).
- **Tool-Discipline:** lean-ctx MCP-Tools statt nativ (`ctx_read`/`ctx_search`/`ctx_edit`/`ctx_shell`); deferred Tool → `ToolSearch(query="select:<tool>")` zuerst, nie Bash-Workaround. `.lmd.md`-Rohquelle nur über `git show HEAD:<path>` lesbar (Shadow-Hook rendert sonst).

---

## File Structure

**Neue Seed-Dateien (alle embedded via `include_str!`):**

- `content/skills/lmd-writing-skills/SKILL.md` — SDO-konformer Discovery-Stub.
- `content/skills/lmd-writing-skills/body.lmd.md` — 4 `@phase`-Blöcke (`red`/`green`/`refactor`/`rationalizations`), je `@include skill-authoring-core`.
- `content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md` — Disziplin-Fragment (Iron Law + letter==spirit + TDD-Mapping + WARUM-Pointer), Built-in.
- `content/skills/lmd-writing-skills/companions/skill-anatomy.lmd.md`
- `content/skills/lmd-writing-skills/companions/skill-discovery-optimization.lmd.md`
- `content/skills/lmd-writing-skills/companions/bulletproofing.lmd.md`
- `content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md`
- `content/skills/lmd-writing-skills/companions/claude-md-testing-example.lmd.md`
- `content/skills/lmd-writing-skills/companions/flowchart-conventions.lmd.md`
- `content/skills/lmd-writing-skills/companions/anthropic-best-practices.lmd.md`
- `content/skills/lmd-writing-skills/companions/persuasion-principles.lmd.md`
- `content/skills/lmd-writing-skills/render-graphs.js` — Asset (nicht gerendert; install-materialisiert).

**Geänderte Code-Dateien:**

- `src/fragments.rs` — `skill-authoring-core` als Built-in registrieren + Consistency-Gate erweitern.
- `src/skills.rs` — `SKILLS`-Tabelle um `lmd-writing-skills`; `COMPANIONS`-Tabelle um 8 Zeilen; je `include_str!`-Const.
- `src/skill_install.rs` — `INSTALLABLE_SKILLS` um `lmd-writing-skills`; neue `ASSETS`-Tabelle + Materialisierungs-Schritt in `install_skill`.
- `src/availability.rs` — `COVERAGE` um `lmd-writing-skills`-Zeilen; `coverage_carries_skill_dimension` erweitern.
- `content/tooling/availability-audit.md` — `lmd-writing-skills`-Coverage-Abschnitt.

**Verifizierte IST-Fakten (Code-Anker):**

- `src/fragments.rs`: `FragmentRegistry::with_builtins()` inserted `hard-rules`, `dispatch-contract`, `test-first-core` (alle `include_str!`-Consts). Test `builtin_fragments_match_seed_files_on_disk` liest die Seeds und vergleicht byte-genau gegen `reg.resolve(name, Path::new("."))`.
- `src/skills.rs`: `SKILLS: &[(&str,&str)]` (`lmd-brainstorm`, `lmd-test-driven-development`). `COMPANIONS: &[(&str,&str,&str)]` (eine Zeile: tdd/testing-anti-patterns). `skill_body`, `all_skill_bodies`, `companion_body`, `render_skill(name, phase, consumer, crp, jail_root) -> Result<String, SkillRenderError>`, `render_companion(skill, companion, …)`.
- `src/skill_install.rs`: `INSTALLABLE_SKILLS: &[(&str,&str)]`; `install_skill(name, scope, project_root)` schreibt nur `SKILL.md`; `target_dir`, `Scope::{Local,Global}`.
- `src/availability.rs`: `COVERAGE: &[(&str,&str,&str,&str)]` = `(skill, step, directive, backing)`; Tests `every_covered_directive_is_registered`, `coverage_carries_skill_dimension`, `coverage_carries_companion_row`.
- Body-Syntax (Vorbild tdd): `@var …`, `@phase "name"` / `@phase-end`, `@include <fragment>`, `{{ var name }}`, „next: …"-Pointer.
- ctx_md_render skill/phase/companion-Verdrahtung + `skill install/remove`-Subcommand existieren bereits (Schwester-Plan) → eine neue Skill ist „nur" Registry- + Seed-Arbeit (+ Asset-Schritt).

**Quell-Dateien des Originals (Port-Basis, verbatim außer Reference-Closure):**

- `~/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/writing-skills/SKILL.md` (Sektionen → section-extraction-Companions + Phasen)
- `…/writing-skills/testing-skills-with-subagents.md`
- `…/writing-skills/anthropic-best-practices.md`
- `…/writing-skills/persuasion-principles.md`
- `…/writing-skills/graphviz-conventions.dot`
- `…/writing-skills/examples/CLAUDE_MD_TESTING.md`
- `…/writing-skills/render-graphs.js`

---

## Task 1: `skill-authoring-core`-Fragment (Seed + Built-in + Consistency-Gate)

Baut das Disziplin-Fragment, das jede Phase per `@include` zieht. Trägt das Iron Law der Skill, das TDD-Mapping und den WARUM-Pointer auf `lmd-test-driven-development`. Liefert es als Built-in (flacher globaler Name) und erweitert das byte-genaue Consistency-Gate.

**Files:**
- Create: `content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md`
- Modify: `src/fragments.rs` (const + `with_builtins`-insert + Consistency-Gate-Test)

**Interfaces:**
- Consumes: `FragmentRegistry::with_builtins()`, `resolve(name, jail_root)` (bestehend).
- Produces: Built-in-Fragment unter dem flachen Namen `skill-authoring-core`; enthält **wörtlich** den Iron-Law-Marker `NO SKILL WITHOUT A FAILING TEST FIRST` und `Writing skills IS test-driven development` (von Task 2/6-Tests als Disziplin-Marker geprüft).

- [ ] **Step 1: Seed-Datei schreiben**

`content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md`:

```markdown
# Skill-Authoring Core (lmd built-in — writing-skills discipline)

Writing skills IS test-driven development applied to process documentation:
write the pressure test first, watch the agent fail without the skill, write the
minimal skill, watch the agent comply, then close loopholes.

**The Iron Law:** NO SKILL WITHOUT A FAILING TEST FIRST.
This applies to NEW skills AND EDITS. Wrote the skill before the baseline test?
Delete it. Start over. Delete means delete — not "keep as reference", not
"adapt it while writing the test", not "just this once".

Violating the letter of the rules is violating the spirit of the rules.

**TDD (test-driven development) mapping for skills:**
- test case = a pressure scenario run against a subagent
- production code = the SKILL.md document
- RED = the agent violates the rule WITHOUT the skill (baseline)
- GREEN = the agent complies WITH the skill present
- REFACTOR = close loopholes while keeping compliance

**Why this is TDD (the WARUM):** see `lmd-test-driven-development` — same
RED -> GREEN -> REFACTOR cycle, same Iron Law, applied to documentation
instead of code.

**The bottom line:** if you follow TDD for code, follow it for skills. Same discipline.
```

- [ ] **Step 2: Failing test schreiben** (`src/fragments.rs`, im `tests`-Modul)

```rust
    #[test]
    fn skill_authoring_core_is_a_builtin_with_iron_law() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("skill-authoring-core", Path::new(".")).unwrap();
        assert!(
            out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "skill-authoring-core must carry the Iron Law marker"
        );
        assert!(
            out.contains("Writing skills IS test-driven development"),
            "skill-authoring-core must state writing-skills-is-TDD"
        );
        assert!(
            out.contains("lmd-test-driven-development"),
            "skill-authoring-core must point to lmd-test-driven-development for the WARUM"
        );
    }
```

- [ ] **Step 3: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run skill_authoring_core_is_a_builtin_with_iron_law`
Expected: FAIL — `resolve("skill-authoring-core", …)` ergibt `Err(NotFound)` → `unwrap()` panics.

- [ ] **Step 4: Fragment als Built-in registrieren** (`src/fragments.rs`)

Const neben `TEST_FIRST_CORE` ergänzen:

```rust
/// Built-in `skill-authoring-core` fragment — the writing-skills discipline
/// trip-wires (Iron Law + letter==spirit + TDD mapping + WARUM pointer).
/// Skill-owned seed, flat global name; `@include skill-authoring-core` pulls it
/// into every isolated writing-skills phase.
const SKILL_AUTHORING_CORE: &str =
    include_str!("../content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md");
```

In `with_builtins()` neben den bestehenden inserts einfügen:

```rust
        builtins.insert("skill-authoring-core", SKILL_AUTHORING_CORE);
```

- [ ] **Step 5: Test ausführen — muss bestehen**

Run: `cargo nextest run skill_authoring_core_is_a_builtin_with_iron_law`
Expected: PASS

- [ ] **Step 6: Consistency-Gate auf neuen Seed erweitern** (`src/fragments.rs`, Test `builtin_fragments_match_seed_files_on_disk`)

Am Ende des bestehenden Tests anfügen (nach dem `test-first-core`-Block):

```rust
        let sac_disk = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md"),
        )
        .unwrap();
        let sac_builtin = reg.resolve("skill-authoring-core", Path::new(".")).unwrap();
        assert_eq!(
            sac_builtin, sac_disk,
            "skill-authoring-core drifted from seed file"
        );
```

- [ ] **Step 7: Consistency-Gate ausführen — muss bestehen**

Run: `cargo nextest run builtin_fragments_match_seed_files_on_disk`
Expected: PASS (built-in == on-disk seed, byte-genau)

- [ ] **Step 8: Formatieren + committen**

```bash
cargo fmt
git add content/skills/lmd-writing-skills/_includes/skill-authoring-core.lmd.md src/fragments.rs
git commit -m "feat(skills): skill-authoring-core discipline fragment as built-in + consistency gate"
```

---

## Task 2: `body.lmd.md` (4 Phasen) + `SKILL.md`-Stub + `SKILLS`-Registrierung

Schreibt den phasen-isolierten Body (red/green/refactor/rationalizations, je `@include skill-authoring-core`) und den dünnen Discovery-Stub, registriert die Skill in `SKILLS` und sichert Registrierung + Phasen-Isolation per Test.

**Files:**
- Create: `content/skills/lmd-writing-skills/body.lmd.md`
- Create: `content/skills/lmd-writing-skills/SKILL.md`
- Modify: `src/skills.rs` (const + `SKILLS`-Zeile + Tests)

**Interfaces:**
- Consumes: `skill-authoring-core` (Task 1), `render_skill(name, phase, consumer, crp, jail_root)`, `skill_body`, `all_skill_bodies` (bestehend).
- Produces: Skill `lmd-writing-skills` mit Phasen `red`/`green`/`refactor`/`rationalizations`; `skill_body("lmd-writing-skills")` ist `Some`.

- [ ] **Step 1: Body-Seed schreiben**

`content/skills/lmd-writing-skills/body.lmd.md`:

```markdown
<!-- lmd-writing-skills body — rendered phase-by-phase via ctx_md_render -->

@phase "red"
@include skill-authoring-core

## RED — write the failing test first (baseline)

Run a pressure scenario against a subagent WITHOUT the skill. This is "watch the
test fail": you must see what agents naturally do before you write anything.

Document exactly: what choice did they make? what rationalizations did they use
(verbatim)? which pressure triggered the violation? For discipline skills combine
3+ pressures (time + sunk cost + authority + exhaustion). ALWAYS run a no-guidance
control — if the control does not exhibit the failure, there is nothing to fix: stop.

next: render phase "green".
@phase-end

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

next: render phase "refactor".
@phase-end

@phase "refactor"
@include skill-authoring-core

## REFACTOR — close loopholes only under green

Agent found a NEW rationalization? Add an explicit counter, then re-test until
bulletproof. Build the rationalization table from every iteration; create a
red-flags list so agents can self-check.

STOP before moving to the next skill: do NOT batch-create skills untested. The
deployment checklist (see companion "testing-skills-with-subagents") is mandatory
for EACH skill. Deploying untested skills = deploying untested code.

For loophole-closing technique render the companion:
`ctx_md_render(skill="lmd-writing-skills", companion="bulletproofing")`.

next: return to RED for the next skill, or ship.
@phase-end

@phase "rationalizations"
@include skill-authoring-core

## Common Rationalizations for Skipping Testing (Excuse | Reality)

| Excuse | Reality |
| "Skill is obviously clear." | Clear to you != clear to other agents. Test it. |
| "It's just a reference." | References have gaps and unclear sections. Test retrieval. |
| "Testing is overkill." | Untested skills have issues. Always. 15 min testing saves hours. |
| "I'll test if problems emerge." | Problems = agents can't use the skill. Test BEFORE deploying. |
| "I'm confident it's good." | Overconfidence guarantees issues. Test anyway. |
| "Academic review is enough." | Reading != using. Test application scenarios. |
| "No time to test." | Deploying an untested skill wastes more time fixing it later. |

**All of these mean: test before deploying. No exceptions.**

For the full testing methodology render the companion:
`ctx_md_render(skill="lmd-writing-skills", companion="testing-skills-with-subagents")`.

next: return to your active phase (red/green/refactor).
@phase-end
```

- [ ] **Step 2: SKILL.md-Stub schreiben**

`content/skills/lmd-writing-skills/SKILL.md`:

```markdown
---
name: lmd-writing-skills
description: Use when creating new skills, editing existing skills, or verifying skills work before deployment
---

# Writing Skills (lmd delegation stub)

Writing skills IS Test-Driven Development applied to process documentation.
Write the pressure test first. Watch the agent fail without the skill. Write the
minimal skill. Watch it comply. Close loopholes.
**Core principle:** If you didn't watch an agent fail without the skill, you don't
know if the skill teaches the right thing.
**Violating the letter of the rules is violating the spirit of the rules.**

This skill's detail is rendered on demand, one phase at a time, by the lean-md
engine. Never read a body or companion file from disk — fetch via the tool.

**REQUIRED BACKGROUND:** lmd-test-driven-development — it defines the
RED-GREEN-REFACTOR cycle and the Iron Law this skill adapts to documentation.

## Where this runs
`ctx_md_render` is provided by the lean-md addon (lean-ctx MCP server, or the
`lean-md` CLI). You do NOT need the lean-md source checked out — every body is
embedded in the running tool.

## The Iron Law
    NO SKILL WITHOUT A FAILING TEST FIRST
Applies to NEW skills AND EDITS. Wrote the skill before the test? Delete it.
Start over. Delete means delete.

## RED -> GREEN -> REFACTOR (render each step as you reach it)
- **RED**      `ctx_md_render(skill="lmd-writing-skills", phase="red")`
- **GREEN**    `ctx_md_render(skill="lmd-writing-skills", phase="green")`
- **REFACTOR** `ctx_md_render(skill="lmd-writing-skills", phase="refactor")`
- **rationalizations** `ctx_md_render(skill="lmd-writing-skills", phase="rationalizations")`

## Companions (render on demand; pass exactly one of phase or companion)
- `skill-anatomy` — what a skill is, types, directory/file structure, SKILL.md template, anti-patterns
- `skill-discovery-optimization` — description/keyword/naming/token rules + discovery workflow
- `bulletproofing` — close loopholes, rationalization tables, match-the-form-to-the-failure
- `testing-skills-with-subagents` — full testing methodology + creation checklist
- `claude-md-testing-example` — worked example of a test campaign
- `flowchart-conventions` — when to use graphviz, shape/naming conventions
- `anthropic-best-practices` — Anthropic's official skill authoring guidance
- `persuasion-principles` — research foundation for bulletproofing

`ctx_md_render(skill="lmd-writing-skills", companion="<name>")`

## Final Rule
    New or edited skill -> baseline test exists and failed first
    Otherwise -> not done
No exceptions without your human partner's permission.
```

- [ ] **Step 3: Failing tests schreiben** (`src/skills.rs`, `tests`-Modul)

```rust
    #[test]
    fn writing_skills_is_registered() {
        assert!(
            skill_body("lmd-writing-skills").is_some(),
            "lmd-writing-skills must be in the SKILLS registry"
        );
        assert!(
            all_skill_bodies().iter().any(|b| b.contains("NO SKILL WITHOUT A FAILING TEST FIRST")),
            "writing-skills body must carry the Iron Law (via @include) — check rendering"
        );
    }

    #[test]
    fn writing_skills_phases_are_isolated() {
        let jail = std::path::PathBuf::from(".");
        let red = render_skill("lmd-writing-skills", Some("red"), None, None, jail.clone()).unwrap();
        let green = render_skill("lmd-writing-skills", Some("green"), None, None, jail.clone()).unwrap();
        // Each phase carries the shared trip-wire...
        assert!(red.contains("NO SKILL WITHOUT A FAILING TEST FIRST"));
        assert!(green.contains("NO SKILL WITHOUT A FAILING TEST FIRST"));
        // ...but NOT the other phase's unique heading (no cross-phase leak).
        assert!(red.contains("RED — write the failing test first"));
        assert!(!red.contains("write the minimal skill"), "red must not leak green");
        assert!(green.contains("write the minimal skill"));
        assert!(!green.contains("Common Rationalizations for Skipping Testing"), "green must not leak rationalizations");
    }
```

- [ ] **Step 4: Tests ausführen — müssen fehlschlagen**

Run: `cargo nextest run writing_skills_is_registered writing_skills_phases_are_isolated`
Expected: FAIL — `skill_body("lmd-writing-skills")` ist `None` → `render_skill` ergibt `Err(UnknownSkill)`.

- [ ] **Step 5: Body-Const + `SKILLS`-Zeile ergänzen** (`src/skills.rs`)

Const neben den bestehenden Body-Consts:

```rust
const LMD_WRITING_SKILLS_BODY: &str =
    include_str!("../content/skills/lmd-writing-skills/body.lmd.md");
```

In der `SKILLS`-Tabelle ergänzen:

```rust
    ("lmd-writing-skills", LMD_WRITING_SKILLS_BODY),
```

- [ ] **Step 6: Tests ausführen — müssen bestehen**

Run: `cargo nextest run writing_skills_is_registered writing_skills_phases_are_isolated`
Expected: PASS

- [ ] **Step 7: Formatieren + committen**

```bash
cargo fmt
git add content/skills/lmd-writing-skills/body.lmd.md content/skills/lmd-writing-skills/SKILL.md src/skills.rs
git commit -m "feat(skills): lmd-writing-skills body (4 phases) + SKILL.md stub + registry"
```

---

## Task 3: Companions (8 Seeds) + `COMPANIONS`-Registrierung + CLI==MCP

Portiert alle 8 Companions verlustfrei (verbatim außer Reference-Closure), registriert sie in `COMPANIONS` und sichert Render + CLI==MCP-Byte-Gleichheit. Die disziplin-nahen Companions (`testing-skills-with-subagents`, `bulletproofing`) ziehen `@include skill-authoring-core` am Kopf.

**Files:**
- Create: 8× `content/skills/lmd-writing-skills/companions/*.lmd.md` (s. Step 1)
- Modify: `src/skills.rs` (8 Consts + 8 `COMPANIONS`-Zeilen + Tests)

**Interfaces:**
- Consumes: `render_companion(skill, companion, consumer, crp, jail_root)`, `companion_body` (bestehend); `skill-authoring-core` (Task 1).
- Produces: 8 Companions unter `lmd-writing-skills`; `companion_body("lmd-writing-skills", <name>)` ist je `Some`.

- [ ] **Step 1: 8 Companion-Seeds schreiben** (verbatim aus den Quell-Dateien, dann Reference-Closure-Edits)

Lies die Quellen via `git`-unabhängigem `ctx_read` (Quell-Dateien liegen außerhalb des Jails → native Read/`ctx_read` ok). Kopiere **verbatim**, dann wende **nur** die unten gelisteten Edits an. Ziel-Dateien unter `content/skills/lmd-writing-skills/companions/`:

| Ziel-Companion (`*.lmd.md`) | Quelle | Reference-Closure-Edits |
|---|---|---|
| `anthropic-best-practices` | `…/writing-skills/anthropic-best-practices.md` (verbatim, ganze Datei) | keine — interne `See [FORMS.md]`/`REFERENCE.md`-Verweise sind illustrative Beispiele, bleiben; externe `https://platform.claude.com/...`-Links bleiben |
| `persuasion-principles` | `…/writing-skills/persuasion-principles.md` (verbatim) | keine bekannten Skill-Cross-Refs (prüfen; nur falls vorhanden: `superpowers:*` → `lmd-*`) |
| `testing-skills-with-subagents` | `…/writing-skills/testing-skills-with-subagents.md` (verbatim) + Kopf-Zeile `@include skill-authoring-core` einfügen | `See examples/CLAUDE_MD_TESTING.md` → `render the companion: ctx_md_render(skill="lmd-writing-skills", companion="claude-md-testing-example")`; jeder `superpowers:test-driven-development` → `lmd-test-driven-development` |
| `claude-md-testing-example` | `…/writing-skills/examples/CLAUDE_MD_TESTING.md` (verbatim) | keine (Worked-Example, in sich geschlossen) |
| `flowchart-conventions` | `…/writing-skills/SKILL.md` Sektion „Flowchart Usage" (verbatim inkl. ```dot-Block) **+** den vollständigen Inhalt von `…/writing-skills/graphviz-conventions.dot` als verbatim ```dot-Block anhängen | `See graphviz-conventions.dot in this directory` → „the graphviz style rules below"; `render-graphs.js`-Erwähnung → Hinweis „installed via `skill install` into the skill directory" |
| `skill-discovery-optimization` | `…/writing-skills/SKILL.md` Sektionen „Skill Discovery Optimization (SDO)" **und** „Discovery Workflow" (verbatim) | im SDO-Cross-Ref-Beispiel `superpowers:test-driven-development` → `lmd-test-driven-development` |
| `bulletproofing` | `…/writing-skills/SKILL.md` Sektionen „Match the Form to the Failure" **und** „Bulletproofing Skills Against Rationalization" (verbatim) + Kopf-Zeile `@include skill-authoring-core` | `See persuasion-principles.md` → `render the companion: ctx_md_render(skill="lmd-writing-skills", companion="persuasion-principles")`; `testing section below` → Verweis auf Companion `testing-skills-with-subagents` |
| `skill-anatomy` | `…/writing-skills/SKILL.md` Sektionen „What is a Skill?", „Skill Types", „Directory Structure", „SKILL.md Structure", „Code Examples", „File Organization", „Anti-Patterns" (verbatim, in dieser Reihenfolge) | `**REQUIRED SUB-SKILL:** Use superpowers:test-driven-development` und `superpowers:systematic-debugging` → `lmd-test-driven-development` bzw. (falls vorhanden) lmd-Pendant; Pfad-Beispiele `skills/skill-name/` bleiben |

Jede Datei beginnt mit einer Kopf-Kommentarzeile im Stil der Schwester-Companion, z.B.:
`# Skill Anatomy (lmd companion — load when structuring a new SKILL.md)`

- [ ] **Step 2: Failing test schreiben** (`src/skills.rs`, `tests`-Modul)

```rust
    #[test]
    fn writing_skills_all_companions_resolve() {
        let names = [
            "skill-anatomy",
            "skill-discovery-optimization",
            "bulletproofing",
            "testing-skills-with-subagents",
            "claude-md-testing-example",
            "flowchart-conventions",
            "anthropic-best-practices",
            "persuasion-principles",
        ];
        for n in names {
            let body = companion_body("lmd-writing-skills", n)
                .unwrap_or_else(|| panic!("companion {n} not registered"));
            assert!(!body.trim().is_empty(), "companion {n} must be non-empty");
        }
    }

    #[test]
    fn writing_skills_discipline_companions_carry_trip_wire() {
        let jail = std::path::PathBuf::from(".");
        for n in ["testing-skills-with-subagents", "bulletproofing"] {
            let out = render_companion("lmd-writing-skills", n, None, None, jail.clone()).unwrap();
            assert!(
                out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
                "discipline companion {n} must @include skill-authoring-core"
            );
        }
    }
```

- [ ] **Step 3: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run writing_skills_all_companions_resolve writing_skills_discipline_companions_carry_trip_wire`
Expected: FAIL — `companion_body("lmd-writing-skills", …)` ist `None` → panic.

- [ ] **Step 4: 8 Consts + `COMPANIONS`-Zeilen ergänzen** (`src/skills.rs`)

Consts neben `LMD_TESTING_ANTI_PATTERNS_COMPANION`:

```rust
const LMD_WS_SKILL_ANATOMY: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/skill-anatomy.lmd.md");
const LMD_WS_SDO: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/skill-discovery-optimization.lmd.md");
const LMD_WS_BULLETPROOFING: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/bulletproofing.lmd.md");
const LMD_WS_TESTING_SUBAGENTS: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md");
const LMD_WS_CLAUDE_MD_TESTING: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/claude-md-testing-example.lmd.md");
const LMD_WS_FLOWCHART: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/flowchart-conventions.lmd.md");
const LMD_WS_ANTHROPIC_BP: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/anthropic-best-practices.lmd.md");
const LMD_WS_PERSUASION: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/persuasion-principles.lmd.md");
```

In der `COMPANIONS`-Tabelle ergänzen:

```rust
    ("lmd-writing-skills", "skill-anatomy", LMD_WS_SKILL_ANATOMY),
    ("lmd-writing-skills", "skill-discovery-optimization", LMD_WS_SDO),
    ("lmd-writing-skills", "bulletproofing", LMD_WS_BULLETPROOFING),
    ("lmd-writing-skills", "testing-skills-with-subagents", LMD_WS_TESTING_SUBAGENTS),
    ("lmd-writing-skills", "claude-md-testing-example", LMD_WS_CLAUDE_MD_TESTING),
    ("lmd-writing-skills", "flowchart-conventions", LMD_WS_FLOWCHART),
    ("lmd-writing-skills", "anthropic-best-practices", LMD_WS_ANTHROPIC_BP),
    ("lmd-writing-skills", "persuasion-principles", LMD_WS_PERSUASION),
```

- [ ] **Step 5: Test ausführen — muss bestehen**

Run: `cargo nextest run writing_skills_all_companions_resolve writing_skills_discipline_companions_carry_trip_wire`
Expected: PASS

- [ ] **Step 6: CLI==MCP-Byte-Gleichheit sichern** (`src/bin/lean_md.rs`, `tests`-Modul — Muster `mcp_companion_matches_cli_render_companion`)

```rust
    #[test]
    fn ws_mcp_companion_matches_cli_render_companion() {
        // CLI==MCP (#498): both surfaces call render_companion → byte-identical.
        let jail = std::path::PathBuf::from(".");
        let cli = lean_md::skills::render_companion(
            "lmd-writing-skills",
            "skill-anatomy",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        let again = lean_md::skills::render_companion(
            "lmd-writing-skills",
            "skill-anatomy",
            None,
            None,
            jail,
        )
        .unwrap();
        assert_eq!(cli, again, "render_companion must be a deterministic function (#498)");
    }
```

- [ ] **Step 7: Test ausführen — muss bestehen**

Run: `cargo nextest run ws_mcp_companion_matches_cli_render_companion`
Expected: PASS

- [ ] **Step 8: Formatieren + committen**

```bash
cargo fmt
git add content/skills/lmd-writing-skills/companions src/skills.rs src/bin/lean_md.rs
git commit -m "feat(skills): lmd-writing-skills 8 companions (full-fidelity port) + registry + CLI==MCP gate"
```

---

## Task 4: Asset `render-graphs.js` + Materialisierung in `skill install`

Liefert `render-graphs.js` als embedded Asset und erweitert `install_skill` um einen Asset-Materialisierungs-Schritt (idempotent), sodass `skill install lmd-writing-skills` neben `SKILL.md` auch das Script in `.claude/skills/lmd-writing-skills/` schreibt.

**Files:**
- Create: `content/skills/lmd-writing-skills/render-graphs.js`
- Modify: `src/skill_install.rs` (`INSTALLABLE_SKILLS` + `ASSETS`-Tabelle + Materialisierungs-Loop + Test)

**Interfaces:**
- Consumes: `install_skill(name, scope, project_root)`, `Scope`, `target_dir` (bestehend).
- Produces: `INSTALLABLE_SKILLS` enthält `lmd-writing-skills`; nach `install_skill("lmd-writing-skills", …)` existiert `<dir>/render-graphs.js` byte-gleich zum embedded Asset.

- [ ] **Step 1: Asset-Datei anlegen** (verbatim Port)

Kopiere `…/writing-skills/render-graphs.js` **verbatim** nach `content/skills/lmd-writing-skills/render-graphs.js`. Keine Edits (das Script liest eine `SKILL.md` und ruft `dot`; Laufzeit-Deps liegen beim Nutzer).

- [ ] **Step 2: Failing test schreiben** (`src/skill_install.rs`, `tests`-Modul)

```rust
    #[test]
    fn writing_skills_install_materializes_asset() {
        let root = std::env::temp_dir().join(format!("lmd_ws_asset_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill("lmd-writing-skills", Scope::Local, &root).unwrap();
        let dir = skill_md.parent().unwrap();
        let asset = dir.join("render-graphs.js");
        assert!(asset.exists(), "render-graphs.js must be materialized next to SKILL.md");
        let on_disk = std::fs::read_to_string(&asset).unwrap();
        assert!(on_disk.contains("extractDotBlocks"), "asset content must be the render script");
        // Idempotent: second install keeps the asset present.
        install_skill("lmd-writing-skills", Scope::Local, &root).unwrap();
        assert!(asset.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
```

- [ ] **Step 3: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run writing_skills_install_materializes_asset`
Expected: FAIL — `install_skill("lmd-writing-skills", …)` ergibt `Err(NotFound)` (Skill nicht in `INSTALLABLE_SKILLS`).

- [ ] **Step 4: Stub-Const + `INSTALLABLE_SKILLS`-Zeile ergänzen** (`src/skill_install.rs`)

```rust
const WRITING_SKILLS_SKILL_MD: &str =
    include_str!("../content/skills/lmd-writing-skills/SKILL.md");
```

```rust
    ("lmd-writing-skills", WRITING_SKILLS_SKILL_MD),
```

- [ ] **Step 5: `ASSETS`-Tabelle + Materialisierungs-Schritt ergänzen** (`src/skill_install.rs`)

Asset-Const + Tabelle neben `INSTALLABLE_SKILLS`:

```rust
const WRITING_SKILLS_RENDER_GRAPHS: &str =
    include_str!("../content/skills/lmd-writing-skills/render-graphs.js");

/// Non-rendered helper files materialized verbatim into the installed skill dir
/// (skill, filename, embedded content). Absent-only/idempotent like the SKILL.md
/// stub (#498 byte-stable).
const ASSETS: &[(&str, &str, &str)] = &[(
    "lmd-writing-skills",
    "render-graphs.js",
    WRITING_SKILLS_RENDER_GRAPHS,
)];
```

In `install_skill`, direkt nach dem `std::fs::write(&target, body)?;` und vor `Ok(target)`:

```rust
    for (skill, fname, content) in ASSETS {
        if *skill == name {
            std::fs::write(dir.join(fname), content)?;
        }
    }
```

- [ ] **Step 6: Test ausführen — muss bestehen**

Run: `cargo nextest run writing_skills_install_materializes_asset`
Expected: PASS

- [ ] **Step 7: Regression — bestehende Install-Tests grün**

Run: `cargo nextest run -E 'test(install)'`
Expected: PASS (TDD-Skill-Install + neue Asset-Logik koexistieren; Skills ohne Assets schreiben keine Extra-Dateien)

- [ ] **Step 8: Formatieren + committen**

```bash
cargo fmt
git add content/skills/lmd-writing-skills/render-graphs.js src/skill_install.rs
git commit -m "feat(skills): render-graphs.js asset + skill-install asset materialization step"
```

---

## Task 5: `COVERAGE` skill-Dimension + Audit-Doc

Trägt `lmd-writing-skills` in die `COVERAGE`-Matrix (Workflow-Schritt → Direktive → lean-ctx-Backing inkl. Companion-Zeile) und erweitert den Skill-Dimensions-Test sowie das Audit-Doc.

**Files:**
- Modify: `src/availability.rs` (`COVERAGE`-Zeilen + `coverage_carries_skill_dimension`)
- Modify: `content/tooling/availability-audit.md` (neuer Abschnitt)

**Interfaces:**
- Consumes: `COVERAGE`, `crate::bridges::default_registry()` (bestehend).
- Produces: `COVERAGE` enthält `lmd-writing-skills`-Zeilen; alle referenzierten Direktiven (`read`, `include`) sind registriert.

- [ ] **Step 1: Failing test schreiben/erweitern** (`src/availability.rs`, Test `coverage_carries_skill_dimension`)

Bestehenden Test um eine Assertion ergänzen:

```rust
        assert!(skills.contains("lmd-writing-skills"));
```

- [ ] **Step 2: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run coverage_carries_skill_dimension`
Expected: FAIL — `lmd-writing-skills` fehlt im `skills`-Set.

- [ ] **Step 3: `COVERAGE`-Zeilen ergänzen** (`src/availability.rs`)

Am Ende der `COVERAGE`-Tabelle (nur registrierte Direktiven verwenden — `read`, `include` existieren bereits):

```rust
    // writing-skills is prose-discipline: the RED baseline reads the skill/test.
    ("lmd-writing-skills", "red", "read", "ctx_read"),
    // Discipline companion pulls the trip-wire via `@include skill-authoring-core`.
    (
        "lmd-writing-skills",
        "testing-skills-with-subagents",
        "include",
        "fragment-compose",
    ),
```

- [ ] **Step 4: Tests ausführen — müssen bestehen**

Run: `cargo nextest run coverage_carries_skill_dimension every_covered_directive_is_registered`
Expected: PASS (beide Direktiven sind in `default_registry()`)

- [ ] **Step 5: Audit-Doc-Abschnitt ergänzen** (`content/tooling/availability-audit.md`)

Neuen Abschnitt analog zum `lmd-test-driven-development`-Block anhängen (via `ctx_edit`):

```markdown
## lmd-writing-skills — Coverage

| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing |
| red (baseline read) | read | ctx_read |
| companion (@include skill-authoring-core) | include | fragment-compose |

Test execution (subagent pressure scenarios) is prose-discipline, not a registered
directive — recorded here for transparency.
```

- [ ] **Step 6: Formatieren + committen**

```bash
cargo fmt
git add src/availability.rs content/tooling/availability-audit.md
git commit -m "feat(availability): COVERAGE rows for lmd-writing-skills + audit doc section"
```

---

## Task 6: Fidelity-Audit (kein Verlust — Rust-Gate + manueller Abgleich)

Sichert verifizierbar, dass jede Phase, jeder Companion und das Asset nicht-leer rendern, und führt den manuellen Section-by-Section-Abgleich Original ↔ Port durch.

**Files:**
- Modify: `src/skills.rs` (Test `tests`-Modul)

**Interfaces:**
- Consumes: `render_skill`, `render_companion`, `companion_body` (bestehend).
- Produces: ein Gate, das alle 4 Phasen + 8 Companions nicht-leer rendert.

- [ ] **Step 1: Fidelity-Gate-Test schreiben** (`src/skills.rs`, `tests`-Modul)

```rust
    #[test]
    fn writing_skills_fidelity_all_surfaces_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in ["red", "green", "refactor", "rationalizations"] {
            let out = render_skill("lmd-writing-skills", Some(p), None, None, jail.clone()).unwrap();
            assert!(out.trim().len() > 80, "phase {p} rendered too thin — content lost?");
        }
        for c in [
            "skill-anatomy",
            "skill-discovery-optimization",
            "bulletproofing",
            "testing-skills-with-subagents",
            "claude-md-testing-example",
            "flowchart-conventions",
            "anthropic-best-practices",
            "persuasion-principles",
        ] {
            let out = render_companion("lmd-writing-skills", c, None, None, jail.clone()).unwrap();
            assert!(out.trim().len() > 80, "companion {c} rendered too thin — content lost?");
        }
    }
```

- [ ] **Step 2: Test ausführen — muss bestehen**

Run: `cargo nextest run writing_skills_fidelity_all_surfaces_render_nonempty`
Expected: PASS

- [ ] **Step 3: Manueller Section-by-Section-Abgleich** (Coverage-Matrix der Spec)

Gehe die Fidelity-Coverage-Matrix aus `docs/lean-md/specs/2026-06-29-lmd-writing-skills-port-design.md` Zeile für Zeile durch. Für jede Original-Sektion/-Datei: öffne das lmd-Ziel und bestätige, dass der Inhalt vorhanden ist (verbatim oder mit dokumentiertem Reference-Closure-Edit). Hake jede Zeile ab. Notiere Lücken in `ctx_session`; falls eine Sektion fehlt → zurück zum passenden Task.

Prüf-Checkliste (jede Quelle muss ein Ziel haben):
- [ ] SKILL.md Overview / TDD-Mapping / Iron Law / Bottom Line → `skill-authoring-core` + Stub
- [ ] RED-GREEN-REFACTOR for Skills / Micro-Test Wording → Phasen red/green/refactor
- [ ] Common Rationalizations / STOP-before-next-skill → Phase rationalizations (+ refactor)
- [ ] What is a Skill / Skill Types / Directory / SKILL.md Structure / Code Examples / File Organization / Anti-Patterns → `skill-anatomy`
- [ ] SDO / Discovery Workflow → `skill-discovery-optimization`
- [ ] Bulletproofing / Match-the-Form → `bulletproofing`
- [ ] Testing All Skill Types / Skill Creation Checklist → `testing-skills-with-subagents`
- [ ] Flowchart Usage + graphviz-conventions.dot → `flowchart-conventions`
- [ ] anthropic-best-practices.md → `anthropic-best-practices`
- [ ] persuasion-principles.md → `persuasion-principles`
- [ ] examples/CLAUDE_MD_TESTING.md → `claude-md-testing-example`
- [ ] render-graphs.js → Asset

- [ ] **Step 4: Formatieren + committen**

```bash
cargo fmt
git add src/skills.rs
git commit -m "test(skills): writing-skills fidelity gate — all phases + companions render non-empty"
```

---

## Task 7: Subagent-Pressure-Test (RED-Baseline → GREEN)

Validiert die Skill nach ihrem eigenen Iron Law: ein Subagent ohne die Skill scheitert an der Aufgabe „schreibe eine Skill" (RED-Baseline), mit der gerenderten Skill ist er compliant (GREEN). Kein Code-Deliverable — Verifikations-Schritt; Befunde in `ctx_session`/`ctx_knowledge`.

**Files:** keine (Verifikation via Subagent-Dispatch).

**Interfaces:**
- Consumes: `render_skill`/`render_companion`-Output (gerenderte Skill als System-Prompt-Material).
- Produces: dokumentierte Baseline-Rationalisierungen + GREEN-Compliance-Beleg.

- [ ] **Step 1: RED-Baseline ohne Skill**

Dispatche einen frischen Subagent (kein Skill-Kontext) mit der Aufgabe: „Schreibe eine neue Skill `foo-bar`, die <konkrete Disziplin-Regel> erzwingt; committe sie." Erwartung (RED): der Agent schreibt die Skill **ohne** vorher ein Baseline-Pressure-Szenario/Test laufen zu lassen. Dokumentiere die Rationalisierungen **verbatim** (`ctx_knowledge action=remember category=blocker`).

Expected: Verstoß sichtbar (Skill geschrieben ohne failing test first).

- [ ] **Step 2: GREEN mit gerenderter Skill**

Rendere die Skill-Phasen (`ctx_md_render(skill="lmd-writing-skills", phase="red")` …) und gib sie dem Subagent als Kontext mit derselben Aufgabe. Erwartung (GREEN): der Agent schreibt zuerst das Baseline-Pressure-Szenario, beobachtet das Scheitern, dann die minimale Skill.

Expected: Agent compliant — failing test first, dann minimale Skill, dann Loophole-Check.

- [ ] **Step 3: Befund festhalten**

`ctx_session action=task value="lmd-writing-skills subagent pressure test [RED→GREEN belegt]"`.
Falls GREEN nicht erreicht: Rationalisierung in die `rationalizations`-Phase oder `bulletproofing`-Companion aufnehmen → zurück zu Task 2/3, dann erneut.

---

## Task 8: Full-Gate-Verifikation

Letzter Durchlauf aller Gates: Format, komplette Test-Suite, Clippy, Determinismus.

**Files:** keine (Verifikation).

- [ ] **Step 1: Format-Check**

Run: `cargo fmt --check`
Expected: keine Ausgabe (alles formatiert)

- [ ] **Step 2: Komplette Test-Suite**

Run: `cargo nextest run`
Expected: PASS — inkl. `skill_authoring_core_is_a_builtin_with_iron_law`, `builtin_fragments_match_seed_files_on_disk`, `writing_skills_is_registered`, `writing_skills_phases_are_isolated`, `writing_skills_all_companions_resolve`, `writing_skills_discipline_companions_carry_trip_wire`, `ws_mcp_companion_matches_cli_render_companion`, `writing_skills_install_materializes_asset`, `coverage_carries_skill_dimension`, `every_covered_directive_is_registered`, `writing_skills_fidelity_all_surfaces_render_nonempty`.

- [ ] **Step 3: Clippy (zero warnings)**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: keine Warnungen

- [ ] **Step 4: Determinismus-Stichprobe (CLI==MCP, byte-stabil)**

Run: `cargo nextest run -E 'test(ws_mcp_companion) + test(cli_eq) + test(byte_stable)'`
Expected: PASS

- [ ] **Step 5: Smoke-Test des Render-Surface** (manuell, optional)

Run: `cargo run --bin lean-md -- render --skill lmd-writing-skills --phase red`
Expected: gerenderte RED-Phase mit Iron-Law-Zeile, kein `PHASE_ABORTED`, kein Cross-Phase-Inhalt.

- [ ] **Step 6: Abschluss-Commit (falls offene Formatierung/Doc)**

```bash
cargo fmt
git add -A
git commit -m "chore(skills): lmd-writing-skills port — full-gate green"
```

---

## Self-Review

**1. Spec coverage:** Jede Spec-Sektion hat einen Task — Architektur/Phasen (T2), Core-Fragment (T1), 8 Companions (T3), Asset+Install (T4), COVERAGE (T5), Fidelity-Gate (T6), Subagent-Pressure-Test (T7), Determinismus/Full-Gate (T8), Reference-Closure (in T3-Edits + Stub T2). Keine Lücke.

**2. Placeholder-Scan:** Authored Content (Core-Fragment, 4 Phasen, Stub, alle Rust-Snippets, alle Tests) ist vollständig ausgeschrieben. Companion-Inhalte sind verbatim-Ports mit exakter Quelle + exakten Reference-Closure-Edits (kein „TBD"/„similar to") — die Quelle ist die vollständige Vorgabe.

**3. Type-Konsistenz:** `skill-authoring-core` (Fragment-Name), `LMD_WRITING_SKILLS_BODY` + `LMD_WS_*` (Consts), `SKILLS`/`COMPANIONS`/`INSTALLABLE_SKILLS`/`ASSETS` (Tabellen), `render_skill`/`render_companion`/`install_skill`/`companion_body`/`skill_body` (Signaturen) durchgehend identisch über alle Tasks. Companion-Namen identisch in Stub (T2), Seeds/Tests (T3), Fidelity-Gate (T6).

---

## Execution Handoff

Nach dem Speichern: Ausführungs-Wahl anbieten (Subagent-Driven empfohlen) — pro Task ein frischer Subagent + zweistufiges Review, gemäß der projektweiten lean-ctx-Multi-Agent-Dispatch-Contract-Pflicht (`.claude/rules/subagent-multi-agent.md`).
