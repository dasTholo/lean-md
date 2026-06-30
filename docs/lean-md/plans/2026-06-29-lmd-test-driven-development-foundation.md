# lmd-test-driven-development + Skill-Platform-Fundament — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die erste nativ portierte lean-md-Skill `lmd-test-driven-development` (4 phasen-isolierte Render-Blöcke) liefern und dabei das geteilte Skill-Platform-Fundament (Registry, Body-Override, `ctx_md_render`-Skill-Verdrahtung, `skill install/remove`, COVERAGE-skill-Dimension) bauen, das Spec #2/#3 wiederverwenden.

**Architecture:** Skill-Bodies sind binary-embedded (`include_str!`) und werden über `ctx_md_render(skill, phase)` phasenweise gerendert (`capture_phase_bodies` → kein Cross-Phase-Leak). Ein kompaktes `test-first-core`-Fragment wird per `@include` in jede Phase gezogen (Disziplin-Trip-Wires trotz Isolation). `skill install` materialisiert nur den dünnen `SKILL.md`-Stub nach `.claude/skills/` (Discovery), während der schwere Body über das MCP-Tool fließt — embedded oder via `.lean-ctx/lean-md/`-Overlay (D7).

**Tech Stack:** Rust (standalone crate `lean_md`, lib + bin), `cargo nextest`, lean-ctx MCP-Tooling (`ctx_read`/`ctx_search`/`ctx_edit`/`ctx_refactor`), `serde_json` (MCP JSON-RPC).

## Global Constraints

- **Tests:** immer `cargo nextest run`, **nie** `cargo test`. Crate ist standalone; `Cargo.toml` + `src/` liegen im Repo-Root → Kommandos laufen aus dem Repo-Root (kein `cd`, kein `--manifest-path` nötig).
- **Shell:** kein `&&`/`||`/`;`-Chaining — jedes Kommando ist eine eigene Invocation.
- **Vor jedem `git add`** (je geänderte Datei): `cargo fmt`.
- **No worktrees** — direkt auf dem aktuellen Branch `feat-lmd-v2` arbeiten.
- **Determinismus (#498):** Tool-Output ist eine deterministische Funktion von (Inhalt, Mode, CRP, Task) — keine Timestamps/Counter/Random in Output-Bodies; embedded Seeds byte-identisch zur on-disk-Quelle (Fragment-Consistency-Gate muss grün bleiben); `CliBackend` == `McpBackend`.
- **Code/Kommentare:** Englisch. Interaktion/Commits-Prosa: Deutsch mit Umlauten.
- **Naming (E2):** Skill heißt ausgeschrieben `lmd-test-driven-development` (nie `lmd-tdd` — `tdd` kollidiert mit dem lean-ctx-CRP-Modus). Acronym „TDD" im Body **nur** disambiguiert („TDD (test-driven development)"), nie als nacktes Keyword (E10).
- **Tool-Discipline:** lean-ctx MCP-Tools statt nativ (`ctx_read`/`ctx_search`/`ctx_edit`/`ctx_shell`); deferred Tool → `ToolSearch(query="select:<tool>")` zuerst, nie Bash-Workaround.

---

## File Structure

**Neue Seed-Dateien (alle embedded via `include_str!`):**

- `content/skills/lmd-test-driven-development/SKILL.md` — SDO-konformer Discovery-Stub (frontmatter + render-on-invoke-Hinweis).
- `content/skills/lmd-test-driven-development/body.lmd.md` — 4 `@phase`-Blöcke (`red`/`green`/`refactor`/`rationalizations`), jede mit `@include test-first-core`.
- `content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md` — Disziplin-Fragment (Iron Law + Letter==Spirit + Red-Flags), registriert als flacher globaler Built-in.

**Neue Code-Datei:**

- `src/skill_install.rs` — `claude_state_dir()`, `Scope`, `install_skill`/`remove_skill`, `INSTALLABLE_SKILLS`-Tabelle.

**Geänderte Code-Dateien:**

- `src/fragments.rs` — `test-first-core` als Built-in registrieren + Consistency-Gate auf neuen Seed erweitern.
- `src/skills.rs` — `match` → `SKILLS`-Registry-Tabelle; Body-Override (D7) in `render_skill`.
- `src/bin/lean_md.rs` — `ctx_md_render` um `skill`/`phase` (tool_defs + tools/call), CLI `render --skill/--phase`, neuer `skill`-Subcommand.
- `src/lib.rs` — `pub mod skill_install;`.
- `src/availability.rs` — `COVERAGE` 3-Tupel → 4-Tupel `(skill, step, directive, backing)`.
- `content/tooling/availability-audit.md` — `lmd-test-driven-development`-Abschnitt + stale Pfad fixen.
- `.gitignore` — `.lean-ctx/lean-md/` + `.claude/` lokal ignorieren (falls noch nicht).

**Verifizierte IST-Fakten (Code-Anker):**

- `src/skills.rs`: `skill_body(name)` ist `match name { "lmd-brainstorm" => Some(LMD_BRAINSTORM_BODY), _ => None }`. `render_skill(name, phase, consumer, crp, jail_root) -> Result<String, SkillRenderError>` existiert, ist aber **nirgends exponiert** (nur Unit-Tests). `SkillRenderError::{UnknownSkill, PhaseNotFound}`.
- `src/fragments.rs`: `FragmentRegistry::with_builtins()` inserted `hard-rules` + `dispatch-contract` (beide `include_str!`-Consts). Test `builtin_fragments_match_seed_files_on_disk` liest `content/core/*` und vergleicht byte-genau.
- `src/phases.rs`: `capture_phase_bodies(ctx, body)` erfasst pro `@phase "name" … @phase-end` den rohen Body unter `name`; `render_skill` rendert die isolierte Phase via `render_body`.
- `src/availability.rs`: `COVERAGE: &[(&str,&str,&str)]` = `(step, directive, backing)`; Test `every_covered_directive_is_registered` destrukturiert `for (step, directive, backing) in COVERAGE`.
- `src/bin/lean_md.rs`: `RenderArgs { file, consumer, crp, out }`; `tool_defs()` exponiert `ctx_md_render` nur mit `path/content/consumer/crp`. Kein `skill`-Subcommand.
- Brainstorm-Body-Syntax (Vorbild): `@phase "pre-context"\n@include hard-rules\n@include dispatch-contract\n@phase-end` usw.

---

## Task 1: `test-first-core`-Fragment (Seed + Built-in-Registrierung + Consistency-Gate)

Baut das Disziplin-Fragment, das jede TDD-Phase per `@include` zieht. Liefert es als Built-in (flacher globaler Name) und erweitert das byte-genaue Consistency-Gate auf den neuen Seed.

**Files:**
- Create: `content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md`
- Modify: `src/fragments.rs` (const + `with_builtins`-insert + Consistency-Gate-Test)

**Interfaces:**
- Consumes: `FragmentRegistry::with_builtins()`, `resolve(name, jail_root)` (bestehend).
- Produces: Built-in-Fragment unter dem flachen Namen `test-first-core`; enthält **wörtlich** den Iron-Law-Marker `NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST` (von Task 2/3-Tests als Disziplin-Marker geprüft).

- [ ] **Step 1: Seed-Datei schreiben**

`content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md`:

```markdown
# Test-First Core (lmd built-in — TDD (test-driven development) discipline)

This is test-driven development: write the test first, watch it fail, then make it pass.

**The Iron Law:** NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST.
Delete means delete: if you delete a test, you delete the behavior it covered.

Violating the letter of the rules is violating the spirit of the rules.

**Red flags — STOP if you catch yourself thinking:**
- "Code before test" — the test comes first, always.
- "The test passed immediately" — then it never failed; you have no proof it tests anything.
- "I'll test after" — code-after-test is production code without a failing test.
- "Too simple to test" — simple code breaks too.
```

- [ ] **Step 2: Failing test schreiben** (`src/fragments.rs`, im `tests`-Modul)

```rust
    #[test]
    fn test_first_core_is_a_builtin_with_iron_law() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("test-first-core", Path::new(".")).unwrap();
        assert!(
            out.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"),
            "test-first-core must carry the Iron Law marker"
        );
        assert!(
            out.contains("Violating the letter of the rules is violating the spirit"),
            "test-first-core must carry the letter==spirit line"
        );
    }
```

- [ ] **Step 3: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run test_first_core_is_a_builtin_with_iron_law`
Expected: FAIL — `resolve("test-first-core", …)` ergibt `Err(NotFound)` → `unwrap()` panics.

- [ ] **Step 4: Fragment als Built-in registrieren** (`src/fragments.rs`)

Const neben `HARD_RULES`/`DISPATCH_CONTRACT` ergänzen:

```rust
/// Built-in `test-first-core` fragment — the TDD discipline trip-wires
/// (Iron Law + letter==spirit + red flags). Skill-owned seed, flat global name;
/// `@include test-first-core` pulls it into every isolated TDD phase (Spec E5).
const TEST_FIRST_CORE: &str =
    include_str!("../content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md");
```

In `with_builtins()` einfügen:

```rust
        builtins.insert("hard-rules", HARD_RULES);
        builtins.insert("dispatch-contract", DISPATCH_CONTRACT);
        builtins.insert("test-first-core", TEST_FIRST_CORE);
```

- [ ] **Step 5: Test ausführen — muss bestehen**

Run: `cargo nextest run test_first_core_is_a_builtin_with_iron_law`
Expected: PASS

- [ ] **Step 6: Consistency-Gate auf neuen Seed erweitern** (`src/fragments.rs`, Test `builtin_fragments_match_seed_files_on_disk`)

Am Ende des bestehenden Tests anfügen (nach dem `dispatch-contract`-Block):

```rust
        let tfc_disk = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md"),
        )
        .unwrap();
        let tfc_builtin = reg.resolve("test-first-core", Path::new(".")).unwrap();
        assert_eq!(
            tfc_builtin, tfc_disk,
            "test-first-core drifted from seed file"
        );
```

- [ ] **Step 7: Consistency-Gate ausführen — muss bestehen**

Run: `cargo nextest run builtin_fragments_match_seed_files_on_disk`
Expected: PASS (built-in == on-disk seed, byte-genau)

- [ ] **Step 8: Formatieren + committen**

```bash
cargo fmt
git add content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md src/fragments.rs
git commit -m "feat(skills): test-first-core discipline fragment as built-in + consistency gate"
```

---

## Task 2: TDD-Body-Seed + SKILL.md-Stub (4 Phasen, je `@include test-first-core`)

Schreibt die zwei reinen Seed-Dateien (kein Code-Logik): den 4-Phasen-Body und den Discovery-Stub. Beide werden in späteren Tasks via `include_str!` eingebunden — hier nur die Inhalte, plus eine Lese-Konsistenz-Prüfung gegen die spätere Registry (in Task 3).

**Files:**
- Create: `content/skills/lmd-test-driven-development/body.lmd.md`
- Create: `content/skills/lmd-test-driven-development/SKILL.md`

**Interfaces:**
- Consumes: `test-first-core`-Fragment (Task 1), `@phase`/`@phase-end`/`@include`-Direktiven (bestehend).
- Produces: Body mit vier Phasen `red`/`green`/`refactor`/`rationalizations`; phasenspezifische Marker `Verify RED`, `Verify GREEN`, `only under green`, `Common Rationalizations` (von Task 3-Tests geprüft). SKILL.md mit `name: lmd-test-driven-development`.

- [ ] **Step 1: Body-Seed schreiben**

`content/skills/lmd-test-driven-development/body.lmd.md`:

```markdown
<!-- lmd-test-driven-development body — rendered phase-by-phase via ctx_md_render -->

@phase "red"
@include test-first-core

## RED — write the failing test first

Write exactly one failing test that pins the next behavior. Then **Verify RED (mandatory)**:
run `ctx_shell "cargo nextest run"` and confirm the test fails *for the right reason*
(it asserts the missing behavior — not a compile error, not a typo).

Good: the test names the behavior and fails on the assertion.
Bad: it fails only because the symbol does not exist yet — that proves nothing about behavior.
@phase-end

@phase "green"
@include test-first-core

## GREEN — minimal code to pass

Write the least code that makes the failing test pass. Then **Verify GREEN (mandatory)**:
run `ctx_shell "cargo nextest run"` and confirm the test passes.

YAGNI: no speculative parameters, no extra abstraction, no code the test does not demand.
@phase-end

@phase "refactor"
@include test-first-core

## REFACTOR — clean up only under green

Refactor only under green: remove duplication, improve names, extract helpers.
No new behavior here — if you need new behavior, return to RED. Re-run
`ctx_shell "cargo nextest run"` after each change; it must stay green.
@phase-end

@phase "rationalizations"
@include test-first-core

## Common Rationalizations (Excuse | Reality)

| Excuse | Reality |
| "I'll test after." | Code-after-test is production code without a failing test. |
| "It's too simple to break." | Simple code breaks; the test is cheap. |
| "The test passed right away." | It never failed — no evidence it tests anything. |
| "Refactor needs a quick prod tweak." | A prod tweak is new behavior — go back to RED. |

**Why order matters:** the failing test is the only proof the test exercises the behavior.
**When stuck:** shrink the test until one tiny behavior is in scope, then RED → GREEN.

Verification checklist: test written first · RED observed · minimal GREEN · refactor under green.
(For testing anti-patterns, see the companion ported in Spec #2 — `testing-anti-patterns`.)
@phase-end
```

- [ ] **Step 2: SKILL.md-Stub schreiben** (SDO-konform — `description` = nur Trigger)

`content/skills/lmd-test-driven-development/SKILL.md`:

```markdown
---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# lmd-test-driven-development (delegation stub)

This skill's body is rendered on demand, one phase at a time, by the lean-ctx
engine. Do not read a body file from disk — invoke the MCP tool:

    ctx_md_render(skill="lmd-test-driven-development", phase="red")

Phase sequence: red → green → refactor → rationalizations. Render each phase as
you reach it; every phase carries the test-first-core discipline block.
```

- [ ] **Step 3: Strukturelle Selbstkontrolle** (kein Test-Run — reine Seeds; verifiziert in Task 3)

Bestätige per `ctx_search`, dass beide Dateien die erwarteten Marker tragen:

Run: `ctx_search(pattern="@phase|@include test-first-core", path="content/skills/lmd-test-driven-development/body.lmd.md")`
Expected: 4× `@phase "…"`, 4× `@phase-end`, 4× `@include test-first-core`.

- [ ] **Step 4: Committen** (keine Code-Änderung → kein `cargo fmt` nötig)

```bash
git add content/skills/lmd-test-driven-development/body.lmd.md content/skills/lmd-test-driven-development/SKILL.md
git commit -m "feat(skills): lmd-test-driven-development body (4 phases) + SDO-konformer SKILL.md-Stub"
```

---

## Task 3: Skill-Registry (`match` → `SKILLS`-Tabelle) + Phasen-Isolation-Gates

Generalisiert `skill_body()` vom hartkodierten `match` zur Lookup-Tabelle und bindet den TDD-Body via `include_str!` ein. Liefert Gate 1 (Phasen-Isolation alle 4 Phasen), Gate 2 (`test-first-core` in jeder Phase), Gate 3 (Registry-Lookup beider Skills).

**Files:**
- Modify: `src/skills.rs` (Registry-Tabelle + Body-Const + Tests)

**Interfaces:**
- Consumes: `body.lmd.md` (Task 2), `test-first-core` (Task 1), `render_skill(name, phase, consumer, crp, jail_root)` (bestehend).
- Produces: `SKILLS: &[(&str, &str)]`-Tabelle; `skill_body("lmd-test-driven-development")` liefert den embedded Body; `render_skill` rendert jede der 4 Phasen isoliert.

- [ ] **Step 1: Failing tests schreiben** (`src/skills.rs`, `tests`-Modul)

```rust
    #[test]
    fn registry_resolves_both_skills() {
        assert!(skill_body("lmd-brainstorm").is_some());
        assert!(skill_body("lmd-test-driven-development").is_some());
        assert!(skill_body("nope").is_none());
    }

    #[test]
    fn tdd_phases_render_isolated_no_cross_leak() {
        for (phase, marker, foreign) in [
            ("red", "Verify RED", "Common Rationalizations"),
            ("green", "Verify GREEN", "Verify RED"),
            ("refactor", "only under green", "Verify GREEN"),
            ("rationalizations", "Common Rationalizations", "Verify RED"),
        ] {
            let out = render_skill(
                "lmd-test-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(out.contains(marker), "phase {phase} missing its marker: {out}");
            assert!(
                !out.contains(foreign),
                "phase {phase} leaked foreign content '{foreign}': {out}"
            );
        }
    }

    #[test]
    fn every_tdd_phase_includes_test_first_core() {
        for phase in ["red", "green", "refactor", "rationalizations"] {
            let out = render_skill(
                "lmd-test-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(
                out.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"),
                "phase {phase} must @include test-first-core (Iron Law marker): {out}"
            );
        }
    }

    #[test]
    fn tdd_body_matches_seed_file_on_disk() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let disk = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-test-driven-development/body.lmd.md"),
        )
        .unwrap();
        assert_eq!(
            skill_body("lmd-test-driven-development").unwrap(),
            disk,
            "embedded TDD body drifted from seed file"
        );
    }
```

- [ ] **Step 2: Tests ausführen — müssen fehlschlagen**

Run: `cargo nextest run skills::tests`
Expected: FAIL — `skill_body("lmd-test-driven-development")` ist `None` (Registry kennt die Skill nicht).

- [ ] **Step 3: Registry-Tabelle implementieren** (`src/skills.rs`)

Body-Const ergänzen und `match` durch Lookup ersetzen:

```rust
const LMD_BRAINSTORM_BODY: &str = include_str!("../content/skills/lmd-brainstorm/body.lmd.md");
const LMD_TEST_DRIVEN_DEVELOPMENT_BODY: &str =
    include_str!("../content/skills/lmd-test-driven-development/body.lmd.md");

/// Registry of embedded lmd skill bodies (name → binary-embedded body source).
/// Replaces the hardcoded `match` so new skills are a one-line table entry
/// (Spec E4 — companion column deferred to Spec #2).
const SKILLS: &[(&str, &str)] = &[
    ("lmd-brainstorm", LMD_BRAINSTORM_BODY),
    ("lmd-test-driven-development", LMD_TEST_DRIVEN_DEVELOPMENT_BODY),
];

/// Embedded body source for a known lmd skill, or `None` if unknown.
pub fn skill_body(name: &str) -> Option<&'static str> {
    SKILLS.iter().find(|(n, _)| *n == name).map(|(_, body)| *body)
}
```

- [ ] **Step 4: Tests ausführen — müssen bestehen**

Run: `cargo nextest run skills::tests`
Expected: PASS (alle 4 neuen Tests + die bestehenden brainstorm-Tests)

- [ ] **Step 5: Formatieren + committen**

```bash
cargo fmt
git add src/skills.rs
git commit -m "feat(skills): SKILLS registry table + lmd-test-driven-development body wiring (Gates 1-3)"
```

---

## Task 4: Body-Override (D7) — Projekt-Overlay vor embedded Const

`render_skill` löst zuerst einen jailed Projekt-Overlay (`<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md`) auf, sonst den embedded Const — lokale Phasen-Iteration ohne Recompile. Liefert Gate 4.

**Files:**
- Modify: `src/skills.rs` (`render_skill` + Override-Helper + Test)

**Interfaces:**
- Consumes: `skill_body(name)` (Task 3), `crate::pathx::jail_path` (bestehend).
- Produces: Override-fähiges `render_skill` — Overlay-Datei vorhanden → Overlay-Quelle gerendert (Phasen-Isolation läuft auf der Overlay-Quelle); absent → embedded.

- [ ] **Step 1: Failing test schreiben** (`src/skills.rs`, `tests`-Modul)

```rust
    #[test]
    fn body_override_prefers_project_overlay() {
        let root = std::env::temp_dir().join(format!("lmd_body_override_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let overlay_dir = root
            .join(".lean-ctx/lean-md/skills/lmd-test-driven-development");
        std::fs::create_dir_all(&overlay_dir).unwrap();
        std::fs::write(
            overlay_dir.join("body.lmd.md"),
            "@phase \"red\"\nOVERLAY_RED_MARKER\n@phase-end\n",
        )
        .unwrap();

        let out = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("OVERLAY_RED_MARKER"),
            "overlay body must be rendered when present: {out}"
        );
        assert!(
            !out.contains("Verify RED"),
            "embedded body must NOT be used when overlay exists: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn body_override_falls_back_to_embedded_when_absent() {
        // No overlay under this jail root → embedded body is used.
        let root = std::env::temp_dir().join(format!("lmd_no_overlay_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(out.contains("Verify RED"), "embedded body fallback: {out}");
        let _ = std::fs::remove_dir_all(&root);
    }
```

- [ ] **Step 2: Tests ausführen — `body_override_prefers_project_overlay` muss fehlschlagen**

Run: `cargo nextest run body_override`
Expected: `body_override_prefers_project_overlay` FAIL (embedded gerendert → `Verify RED` vorhanden, `OVERLAY_RED_MARKER` fehlt); `body_override_falls_back_to_embedded_when_absent` PASS.

- [ ] **Step 3: Override-Helper + `render_skill`-Auflösung implementieren** (`src/skills.rs`)

Helper über `render_skill` einfügen:

```rust
/// D7 body-override: a jailed project overlay at
/// `<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md` wins over the
/// embedded const, enabling local phase iteration without a recompile.
/// PathJail-bound (no escape outside `jail_root`).
fn overlay_body(name: &str, jail_root: &std::path::Path) -> Option<String> {
    let candidate = jail_root
        .join(".lean-ctx/lean-md/skills")
        .join(name)
        .join("body.lmd.md");
    let resolved = crate::pathx::jail_path(&candidate, jail_root).ok()?;
    if !resolved.exists() {
        return None;
    }
    std::fs::read_to_string(&resolved).ok()
}
```

In `render_skill` den `src`-Bezug ersetzen. Aktuell:

```rust
    let src = skill_body(name).ok_or_else(|| SkillRenderError::UnknownSkill(name.to_string()))?;
    let (mut header, body) = parse_header(src);
```

→ neu (Overlay zuerst, sonst embedded; `body` muss auf einen `&str` zeigen, der lange genug lebt):

```rust
    let owned_overlay = overlay_body(name, &jail_root);
    let src: &str = match owned_overlay.as_deref() {
        Some(s) => s,
        None => skill_body(name).ok_or_else(|| SkillRenderError::UnknownSkill(name.to_string()))?,
    };
    let (mut header, body) = parse_header(src);
```

- [ ] **Step 4: Tests ausführen — müssen bestehen**

Run: `cargo nextest run body_override`
Expected: PASS (beide). Zusätzlich `cargo nextest run skills::tests` → weiterhin grün.

- [ ] **Step 5: Formatieren + committen**

```bash
cargo fmt
git add src/skills.rs
git commit -m "feat(skills): D7 body-override — project overlay precedes embedded const (Gate 4)"
```

---

## Task 5: `ctx_md_render` skill/phase-Verdrahtung (MCP + CLI, byte-stabil)

Exponiert das heute tote `render_skill` über das MCP-Tool und die CLI. Liefert Gate 6 (MCP- und CLI-Pfad rendern identisch; `skill`-Param branched; Fehler → `-32602`).

**Files:**
- Modify: `src/bin/lean_md.rs` (`tool_defs`, `tools/call`-Branch, `RenderArgs`/`parse_render_flags`, `cmd_render`)

**Interfaces:**
- Consumes: `lean_md::skills::{render_skill, SkillRenderError}`, `Consumer`, `CrpMode` (bestehend).
- Produces: `ctx_md_render(skill, phase)` über MCP; `lean-md render --skill <name> [--phase <p>]` über CLI; beide rufen denselben `render_skill` → byte-stabil (#498).

- [ ] **Step 1: Failing test schreiben** (`src/bin/lean_md.rs`, neues/bestehendes `tests`-Modul am Dateiende)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_flags_parse_skill_and_phase() {
        let a = parse_render_flags(&[
            "--skill".to_string(),
            "lmd-test-driven-development".to_string(),
            "--phase".to_string(),
            "red".to_string(),
        ]);
        assert_eq!(a.skill.as_deref(), Some("lmd-test-driven-development"));
        assert_eq!(a.phase.as_deref(), Some("red"));
    }
}
```

- [ ] **Step 2: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run render_flags_parse_skill_and_phase`
Expected: FAIL — `RenderArgs` hat kein `skill`/`phase`-Feld → kompiliert nicht.

- [ ] **Step 3: `RenderArgs` + Flag-Parsing erweitern** (`src/bin/lean_md.rs`)

`RenderArgs` um zwei Felder erweitern:

```rust
#[derive(Debug, Default, PartialEq)]
struct RenderArgs {
    file: Option<String>,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    out: Option<String>,
    skill: Option<String>,
    phase: Option<String>,
}
```

In `parse_render_flags` im `match arg`-Block (vor dem letzten `_ if !arg.starts_with('-') …`-Arm) ergänzen:

```rust
            "--skill" => {
                i += 1;
                a.skill = rest.get(i).cloned();
            }
            "--phase" => {
                i += 1;
                a.phase = rest.get(i).cloned();
            }
```

- [ ] **Step 4: Test ausführen — muss bestehen**

Run: `cargo nextest run render_flags_parse_skill_and_phase`
Expected: PASS

- [ ] **Step 5: CLI-`cmd_render` um Skill-Pfad erweitern** (`src/bin/lean_md.rs`)

Import oben ergänzen:

```rust
use lean_md::skills::{SkillRenderError, render_skill};
```

In `cmd_render` **vor** dem `let Some(file) = a.file …`-Block den Skill-Zweig einfügen:

```rust
    if let Some(skill) = a.skill.as_deref() {
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match render_skill(skill, a.phase.as_deref(), a.consumer, a.crp, jail) {
            Ok(rendered) => match a.out {
                Some(out) => {
                    if let Err(e) = std::fs::write(&out, &rendered) {
                        eprintln!("lean-md render: write {out}: {e}");
                        std::process::exit(1);
                    }
                }
                None => print!("{rendered}"),
            },
            Err(e) => {
                eprintln!("lean-md render: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
```

(`SkillRenderError` implementiert `Display` → `{e}` ergibt `UNKNOWN_SKILL '…'` / `PHASE_NOT_FOUND '…'`.)

- [ ] **Step 6: MCP `tool_defs()` um `skill`/`phase` erweitern** (`src/bin/lean_md.rs`)

In den `properties` von `ctx_md_render` zwei Felder ergänzen:

```rust
                    "skill":    { "type": "string", "description": "Render an embedded lmd skill body by name (alternative to path/content)" },
                    "phase":    { "type": "string", "description": "Render only this named phase of the skill (requires skill)" }
```

- [ ] **Step 7: MCP `tools/call`-Branch verdrahten** (`src/bin/lean_md.rs`)

Im `"ctx_md_render"`-Arm **vor** `match mcp_load_source(&args)` den Skill-Zweig einsetzen:

```rust
                    "ctx_md_render" => {
                        if let Some(skill) = args.get("skill").and_then(Value::as_str) {
                            let phase = args.get("phase").and_then(Value::as_str);
                            let consumer = args.get("consumer").and_then(Value::as_str).and_then(
                                |s| match s.trim() {
                                    "human" => Some(Consumer::Human),
                                    "ai" => Some(Consumer::Ai),
                                    _ => None,
                                },
                            );
                            let crp = args
                                .get("crp")
                                .and_then(Value::as_str)
                                .and_then(|s| CrpMode::parse(s));
                            let jail = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            match render_skill(skill, phase, consumer, crp, jail) {
                                Ok(rendered) => rpc_ok(
                                    &id,
                                    json!({ "content": [{ "type": "text", "text": rendered }] }),
                                ),
                                Err(e) => rpc_err(&id, -32602, &format!("{e}")),
                            }
                        } else {
                            match mcp_load_source(&args) {
                                // … bestehender do_render-Pfad unverändert …
                            }
                        }
                    }
```

(Der bestehende `match mcp_load_source(&args) { Ok(...) => {...} Err(e) => rpc_err(&id, -32602, &e) }`-Block wandert unverändert in den `else`-Zweig.)

- [ ] **Step 8: Byte-Stabilität CLI==Library verifizieren** (`src/bin/lean_md.rs`, `tests`-Modul)

```rust
    #[test]
    fn skill_render_is_byte_stable_and_isolated() {
        let jail = std::path::PathBuf::from(".");
        let a = render_skill("lmd-test-driven-development", Some("green"), None, None, jail.clone()).unwrap();
        let b = render_skill("lmd-test-driven-development", Some("green"), None, None, jail).unwrap();
        assert_eq!(a, b, "render_skill must be deterministic (#498)");
        assert!(a.contains("Verify GREEN"));
        assert!(!a.contains("Verify RED"), "phase isolation in the exposed path");
    }
```

- [ ] **Step 9: Tests ausführen — müssen bestehen**

Run: `cargo nextest run --bin lean-md`
Expected: PASS (Flag-Parsing + Byte-Stabilität). Dann voller Lauf: `cargo nextest run` → grün.

- [ ] **Step 10: Manueller E2E (CLI-Pfad)**

Run: `ctx_shell "cargo run --bin lean-md -- render --skill lmd-test-driven-development --phase red"`
Expected: gerenderte RED-Phase mit „Verify RED" + Iron-Law-Marker, **ohne** „Common Rationalizations".

- [ ] **Step 11: Formatieren + committen**

```bash
cargo fmt
git add src/bin/lean_md.rs
git commit -m "feat(cli/mcp): expose render_skill via ctx_md_render skill/phase + render --skill/--phase (Gate 6)"
```

---

## Task 6: `skill install`/`remove` + `claude_state_dir()` (neues Modul, beide Scopes)

Neues `src/skill_install.rs`: materialisiert den `SKILL.md`-Stub nach `--local` (Default, projekt-relativ) oder `--global` (`claude_state_dir()`). Liefert Gate 5 (Roundtrip beide Scopes, idempotent).

**Files:**
- Create: `src/skill_install.rs`
- Modify: `src/lib.rs` (`pub mod skill_install;`)
- Modify: `src/bin/lean_md.rs` (neuer `skill`-Subcommand + `cmd_skill`)

**Interfaces:**
- Consumes: `INSTALLABLE_SKILLS` (embedded `SKILL.md` via `include_str!`).
- Produces: `claude_state_dir() -> PathBuf`; `enum Scope { Local, Global }`; `install_skill(name, scope, project_root) -> io::Result<PathBuf>`; `remove_skill(name, scope, project_root) -> io::Result<()>`.

- [ ] **Step 1: Modul-Skelett + Failing tests schreiben** (`src/skill_install.rs`)

```rust
//! Skill materialization (Spec §4.6, E7/E11). Writes the thin `SKILL.md` stub
//! (Discovery channel) into `.claude/skills/<name>/`. The heavy body never
//! lands here — it flows through `ctx_md_render` (embedded or `.lean-ctx/lean-md/`
//! overlay). Install home moved into lean-md (Baseline §2.2: lean-ctx installer
//! removed). Opt-in = invocation; `--global|--local` selects the target only.

use std::path::{Path, PathBuf};

const TDD_SKILL_MD: &str =
    include_str!("../content/skills/lmd-test-driven-development/SKILL.md");
const BRAINSTORM_SKILL_MD: &str = include_str!("../content/skills/lmd-brainstorm/SKILL.md");

/// Installable lmd skills (name → embedded `SKILL.md` stub).
pub const INSTALLABLE_SKILLS: &[(&str, &str)] = &[
    ("lmd-test-driven-development", TDD_SKILL_MD),
    ("lmd-brainstorm", BRAINSTORM_SKILL_MD),
];

/// Install target selector (Spec E11). `Local` is the default — env-independent,
/// versionable, team-shareable. `Global` honors `CLAUDE_CONFIG_DIR`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scope {
    Local,
    Global,
}

/// Global Claude state dir (Spec E11/R3): `CLAUDE_CONFIG_DIR` else `~/.claude`.
/// ONLY the global target reacts to `CLAUDE_CONFIG_DIR`.
pub fn claude_state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude")
}

fn skill_md(name: &str) -> Option<&'static str> {
    INSTALLABLE_SKILLS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
}

/// Target dir for a skill under the chosen scope. `--local` is project-relative
/// (env-independent); `--global` is under `claude_state_dir()`.
fn target_dir(name: &str, scope: Scope, project_root: &Path) -> PathBuf {
    match scope {
        Scope::Local => project_root.join(".claude/skills").join(name),
        Scope::Global => claude_state_dir().join("skills").join(name),
    }
}

/// Materialize a skill's `SKILL.md` into the chosen scope. Atomic-ish,
/// idempotent (overwrites the stub — byte-stable content, #498).
pub fn install_skill(name: &str, scope: Scope, project_root: &Path) -> std::io::Result<PathBuf> {
    let body = skill_md(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("unknown installable skill: {name}"),
        )
    })?;
    let dir = target_dir(name, scope, project_root);
    std::fs::create_dir_all(&dir)?;
    let target = dir.join("SKILL.md");
    std::fs::write(&target, body)?;
    Ok(target)
}

/// Remove the lmd-owned skill dir in the chosen scope only. Absent-tolerant.
pub fn remove_skill(name: &str, scope: Scope, project_root: &Path) -> std::io::Result<()> {
    let dir = target_dir(name, scope, project_root);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_state_dir_honors_config_dir() {
        // SAFETY: single-threaded nextest process-per-test isolates env mutation.
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/pinned-claude") };
        assert_eq!(claude_state_dir(), PathBuf::from("/tmp/pinned-claude"));
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
    }

    #[test]
    fn local_install_is_project_relative_and_ignores_config_dir() {
        let root = std::env::temp_dir().join(format!("lmd_install_local_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // A set CLAUDE_CONFIG_DIR must NOT affect the local target.
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/should-be-ignored") };
        let target =
            install_skill("lmd-test-driven-development", Scope::Local, &root).unwrap();
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
        let expected = root
            .join(".claude/skills/lmd-test-driven-development/SKILL.md");
        assert_eq!(target, expected, "local target must be project-relative");
        assert!(target.exists(), "SKILL.md must be written");
        let body = std::fs::read_to_string(&target).unwrap();
        assert!(body.contains("name: lmd-test-driven-development"));
        // Idempotent: second install is fine, file still present.
        install_skill("lmd-test-driven-development", Scope::Local, &root).unwrap();
        assert!(target.exists());
        // Remove takes it away.
        remove_skill("lmd-test-driven-development", Scope::Local, &root).unwrap();
        assert!(!target.exists(), "remove must delete the skill dir");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn global_install_uses_pinned_config_dir() {
        let pin = std::env::temp_dir().join(format!("lmd_install_global_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pin);
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", pin.to_str().unwrap()) };
        let project = std::env::temp_dir().join("lmd_install_global_proj");
        let target =
            install_skill("lmd-test-driven-development", Scope::Global, &project).unwrap();
        let expected = pin.join("skills/lmd-test-driven-development/SKILL.md");
        assert_eq!(target, expected, "global target must be under CLAUDE_CONFIG_DIR");
        assert!(target.exists());
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
        let _ = std::fs::remove_dir_all(&pin);
    }

    #[test]
    fn unknown_skill_install_errors() {
        let root = std::env::temp_dir();
        let err = install_skill("nope", Scope::Local, &root).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
```

> **Hinweis (Rust-Edition):** `std::env::set_var`/`remove_var` sind in Edition 2024 `unsafe` — die `unsafe`-Blöcke oben sind korrekt. Prüfe die Edition in `Cargo.toml`; bei 2021 die `unsafe { … }`-Wrapper entfernen (dann ist der nackte Call korrekt). Die Tests, die `CLAUDE_CONFIG_DIR` pinnen, müssen process-seriell laufen — nextest isoliert per Default Prozess-pro-Test, daher unkritisch.

- [ ] **Step 2: Modul exportieren** (`src/lib.rs`)

Nach `pub mod skills;` (alphabetische Nähe) ergänzen:

```rust
pub mod skill_install;
```

- [ ] **Step 3: Tests ausführen — müssen fehlschlagen, dann nach Modul-Anlage bestehen**

Run: `cargo nextest run skill_install`
Expected: Nach Step 1+2 kompiliert das Modul mit Impl → die Tests sollten **bestehen** (das Modul wurde bereits vollständig geschrieben). Falls die Edition-`unsafe`-Frage auftaucht: Build-Fehler beheben gem. Hinweis, erneut laufen. (Dies ist der „red→green" für ein neues, in sich geschlossenes Modul: zuerst Compile-Fehler ohne `lib.rs`-Export → nach Export grün.)

- [ ] **Step 4: CLI-`skill`-Subcommand verdrahten** (`src/bin/lean_md.rs`)

Import ergänzen:

```rust
use lean_md::skill_install::{Scope, install_skill, remove_skill};
```

In `main()` den `match action` um einen Arm erweitern:

```rust
        "skill" => cmd_skill(&args[1..]),
```

Usage-Text in `main()` um die Zeile ergänzen:

```
                 \n  skill  <install|remove> <name> [--global|--local]
```

`cmd_skill` neu hinzufügen:

```rust
fn cmd_skill(rest: &[String]) {
    let sub = rest.first().map_or("", String::as_str);
    let name = rest.iter().skip(1).find(|a| !a.starts_with('-'));
    let scope = if rest.iter().any(|a| a == "--global") {
        Scope::Global
    } else {
        Scope::Local
    };
    let Some(name) = name else {
        eprintln!("lean-md skill: missing <name>");
        std::process::exit(1);
    };
    let project_root =
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match sub {
        "install" => match install_skill(name, scope, &project_root) {
            Ok(target) => println!("installed {name} → {}", target.display()),
            Err(e) => {
                eprintln!("lean-md skill install: {e}");
                std::process::exit(1);
            }
        },
        "remove" => match remove_skill(name, scope, &project_root) {
            Ok(()) => println!("removed {name}"),
            Err(e) => {
                eprintln!("lean-md skill remove: {e}");
                std::process::exit(1);
            }
        },
        other => {
            eprintln!("lean-md skill: unknown subcommand '{other}' (install|remove)");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 5: Voller Test-Lauf**

Run: `cargo nextest run`
Expected: PASS (alle Module inkl. `skill_install`).

- [ ] **Step 6: Manueller E2E (Install-Roundtrip, lokaler Default-Scope)**

Run: `ctx_shell "cargo run --bin lean-md -- skill install lmd-test-driven-development --local"`
Expected: `installed lmd-test-driven-development → .../.claude/skills/lmd-test-driven-development/SKILL.md`; Datei existiert repo-lokal (kein env-Trick nötig).

- [ ] **Step 7: Formatieren + committen**

```bash
cargo fmt
git add src/skill_install.rs src/lib.rs src/bin/lean_md.rs
git commit -m "feat(install): skill install/remove + claude_state_dir, --local default / --global (Gate 5)"
```

---

## Task 7: `COVERAGE` skill-Dimension + Audit-Doc (E9)

Generalisiert `COVERAGE` vom 3-Tupel `(step, directive, backing)` zum 4-Tupel `(skill, step, directive, backing)`, fügt TDD-Zeilen hinzu und aktualisiert das Audit-Doc (inkl. stale-Pfad-Fix). Liefert Gate 8.

**Files:**
- Modify: `src/availability.rs` (`COVERAGE`-Typ + Zeilen + Test-Destrukturierung)
- Modify: `content/tooling/availability-audit.md` (TDD-Abschnitt + stale Pfad)

**Interfaces:**
- Consumes: `crate::bridges::default_registry()` (bestehend), `reg.get(directive)`.
- Produces: `COVERAGE: &[(&str,&str,&str,&str)]` mit `skill`-Spalte; jede covered Direktive bleibt in `default_registry()` registriert.

- [ ] **Step 1: Failing test schreiben** (`src/availability.rs`, `tests`-Modul)

```rust
    #[test]
    fn coverage_carries_skill_dimension() {
        // Both skills must appear; every TDD row's directive must be registered.
        let skills: std::collections::HashSet<&str> =
            COVERAGE.iter().map(|(skill, _, _, _)| *skill).collect();
        assert!(skills.contains("lmd-brainstorm"));
        assert!(skills.contains("lmd-test-driven-development"));
    }
```

- [ ] **Step 2: Test ausführen — muss fehlschlagen**

Run: `cargo nextest run coverage_carries_skill_dimension`
Expected: FAIL — `COVERAGE`-Tupel ist 3-elementig → `|(skill, _, _, _)|` kompiliert nicht.

- [ ] **Step 3: `COVERAGE` auf 4-Tupel umstellen** (`src/availability.rs`)

```rust
/// (skill, workflow step, lmd directive name as in default_registry, lean-ctx backing).
pub const COVERAGE: &[(&str, &str, &str, &str)] = &[
    ("lmd-brainstorm", "explore", "read", "ctx_read"),
    ("lmd-brainstorm", "explore", "list", "ctx_tree"),
    ("lmd-brainstorm", "explore", "search", "ctx_search"),
    ("lmd-brainstorm", "explore", "find", "ctx_semantic_search"),
    ("lmd-brainstorm", "approaches", "graph", "graph_index"),
    ("lmd-brainstorm", "approaches", "impact", "ctx_impact"),
    ("lmd-brainstorm", "write-spec", "edit", "ctx_edit"),
    ("lmd-brainstorm", "write-spec", "remember", "ctx_knowledge"),
    ("lmd-brainstorm", "self-review", "review", "ctx_review"),
    ("lmd-brainstorm", "handoff", "dispatch", "fragment-compose"),
    ("lmd-brainstorm", "handoff", "handoff", "ctx_handoff"),
    // TDD is prose-discipline + directive-arm: the RED phase reads the test/impl.
    // Test execution (`ctx_shell "cargo nextest run"`) is NOT a registered
    // directive — see GAP_LIST note below.
    ("lmd-test-driven-development", "red", "read", "ctx_read"),
];
```

- [ ] **Step 4: Bestehende Test-Destrukturierung anpassen** (`src/availability.rs`, Test `every_covered_directive_is_registered`)

```rust
        for (skill, step, directive, backing) in COVERAGE {
            assert!(
                reg.get(directive).is_some(),
                "directive '{directive}' (skill={skill}, step={step}, backing={backing}) not in default_registry()"
            );
        }
```

- [ ] **Step 5: GAP_LIST-Notiz für TDD-Test-Run ergänzen** (`src/availability.rs`)

`GAP_LIST` bleibt bei den drei Tools; ergänze einen Doc-Kommentar über `pub const GAP_LIST`:

```rust
/// Tools deliberately outside the brainstorming directive surface. Note: TDD's
/// test execution (`ctx_shell "cargo nextest run"`) is also intentionally NOT a
/// registered directive — it is raw shell, not a code-intel directive (TDD is
/// prose-discipline). Recorded here for transparency, not added to the list.
pub const GAP_LIST: &[&str] = &["ctx_benchmark", "ctx_package", "ctx_provider"];
```

- [ ] **Step 6: Tests ausführen — müssen bestehen**

Run: `cargo nextest run availability`
Expected: PASS (`coverage_carries_skill_dimension`, `every_covered_directive_is_registered`, `gap_list_is_byte_stable`).

- [ ] **Step 7: Audit-Doc aktualisieren** (`content/tooling/availability-audit.md`)

Stale Pfad fixen — `rust/src/lmd/availability.rs` → `src/availability.rs` (im einleitenden Absatz). Danach am Dateiende den TDD-Abschnitt anfügen:

```markdown

## lmd-test-driven-development — Coverage

TDD ist Prosa-Disziplin (phasenweise gerendert), direktiv-arm:

| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing |
| red              | `@read`       | `ctx_read`       |

**Bewusster Gap:** Die Test-Ausführung (`ctx_shell "cargo nextest run"`) ist **keine**
registrierte Direktive — sie läuft als rohes `ctx_shell`, nicht als Code-Intel-Direktive.
RED/GREEN-Verifikation ist Prosa-Anweisung im Body, kein Registry-Eintrag (transparent, kein Loch).
```

- [ ] **Step 8: Voller Test-Lauf**

Run: `cargo nextest run`
Expected: PASS (alle Gates).

- [ ] **Step 9: Formatieren + committen**

```bash
cargo fmt
git add src/availability.rs content/tooling/availability-audit.md
git commit -m "feat(availability): COVERAGE skill dimension + TDD section, stale path fix (Gate 8)"
```

---

## Task 8: `.gitignore`-Härtung + Full-Gate-Verifikation (alle 9 Gates)

Stellt sicher, dass materialisierte Laufzeit-Artefakte nicht eingecheckt werden, und verifiziert die vollständige Gate-Liste aus Spec §5 in einem Durchlauf.

**Files:**
- Modify: `.gitignore` (falls Einträge fehlen)

**Interfaces:**
- Consumes: alle vorherigen Tasks.
- Produces: grüner Full-Run; saubere Worktree-Trennung von Laufzeit-Overlays.

- [ ] **Step 1: `.gitignore`-IST prüfen**

Run: `ctx_read(path=".gitignore", mode=full)`
Expected: feststellen, ob `.lean-ctx/lean-md/` und ein lokales `.claude/` (Skill-Install-Default-Scope) bereits ignoriert sind.

- [ ] **Step 2: Fehlende Einträge ergänzen** (`.gitignore`)

Falls nicht vorhanden, anfügen (nur die fehlenden Zeilen):

```gitignore
# lean-md runtime overlays + materialized skill stubs (Spec §6)
.lean-ctx/lean-md/
/.claude/skills/
```

(Achtung: Repo-eigene, versionierte `.claude/`-Inhalte nicht breit ignorieren — nur den Skill-Install-Default-Scope `/.claude/skills/`.)

- [ ] **Step 3: Format-Gate über alle berührten Rust-Dateien**

Run: `cargo fmt --check`
Expected: keine Ausgabe (alles formatiert). Bei Abweichung: `cargo fmt`, erneut prüfen.

- [ ] **Step 4: Clippy-Gate (Quality-Bar: zero warnings)**

Run: `ctx_shell "cargo clippy --all-targets -- -D warnings"`
Expected: keine Warnings/Errors.

- [ ] **Step 5: Voller Test-Lauf — alle 9 Gates**

Run: `cargo nextest run`
Expected: PASS. Gate-Mapping:
- Gate 1 (Phasen-Isolation 4 Phasen) → `tdd_phases_render_isolated_no_cross_leak`
- Gate 2 (`test-first-core` in jeder Phase) → `every_tdd_phase_includes_test_first_core`
- Gate 3 (Registry beide Skills) → `registry_resolves_both_skills`
- Gate 4 (Body-Override D7) → `body_override_prefers_project_overlay` / `…_falls_back_…`
- Gate 5 (Install beide Scopes) → `local_install_…` / `global_install_…` / `unknown_skill_install_errors`
- Gate 6 (`ctx_md_render` skill/phase byte-stabil) → `render_flags_parse_skill_and_phase` / `skill_render_is_byte_stable_and_isolated`
- Gate 7 (Fragment-Consistency) → `builtin_fragments_match_seed_files_on_disk` / `tdd_body_matches_seed_file_on_disk`
- Gate 8 (COVERAGE↔Audit skill-Dim) → `coverage_carries_skill_dimension` / `every_covered_directive_is_registered`
- Gate 9 (Determinismus #498) → `skill_render_is_byte_stable_and_isolated` + bestehende byte-stable-Tests

- [ ] **Step 6: Manueller E2E-Gesamtfluss** (Spec §6)

```text
1. ctx_shell "cargo run --bin lean-md -- skill install lmd-test-driven-development --local"
   → .claude/skills/lmd-test-driven-development/SKILL.md vorhanden
2. ctx_shell "cargo run --bin lean-md -- render --skill lmd-test-driven-development --phase red"
   → isolierte RED-Phase, „Verify RED" + Iron Law, ohne „Common Rationalizations"
```

Expected: beide Schritte wie beschrieben; keine Timestamps/Counter im Output (#498).

- [ ] **Step 7: Committen**

```bash
git add .gitignore
git commit -m "chore: gitignore lean-md runtime overlays + local skill stubs; full 9-gate verification"
```

---

## Self-Review

**1. Spec coverage:**

- §3.1/§4.3 Skill-Registry → Task 3 ✅
- §3.1/§4.4 Body-Override D7 → Task 4 ✅
- §3.1/§4.5/E8 `ctx_md_render`-Verdrahtung → Task 5 ✅
- §3.1/§4.6/E7/E11 `skill install/remove` + `claude_state_dir()` → Task 6 ✅
- §3.1/§4.7/E9 COVERAGE skill-Dimension + Audit-Doc → Task 7 ✅
- §3.2/§4.1 4-Phasen-Body + `test-first-core` → Task 1 (Fragment) + Task 2 (Body) ✅
- §4.2 SKILL.md-Stub SDO-konform → Task 2 ✅
- §5 Gates 1–9 → über Tasks verteilt, gesammelt verifiziert in Task 8 ✅
- E5 `_includes/`-Konvention + flacher Built-in-Name → Task 1 ✅
- E10 Keyword-Coverage / TDD nur disambiguiert → Task 1 (test-first-core) + Task 2 (Body) ✅
- §6 lokaler Test-Flow → Task 6/8 E2E ✅

**2. Placeholder-Scan:** Jeder Code-Step enthält vollständigen Code; keine „TBD"/„add error handling"/„similar to Task N". ✅

**3. Type-Konsistenz:** `render_skill(name, phase: Option<&str>, consumer: Option<Consumer>, crp: Option<CrpMode>, jail_root: PathBuf)` durchgängig (Task 3/4/5). `Scope`/`install_skill`/`remove_skill`/`claude_state_dir` konsistent (Task 6). `COVERAGE` 4-Tupel `(skill, step, directive, backing)` in Const + Test (Task 7). `skill_body(name) -> Option<&'static str>` unverändert in Signatur, nur Impl (Task 3). ✅

**Bewusste Bootstrap-Notiz (R2):** Spec #1 wird mit der *superpowers*-`writing-skills`/`test-driven-development` autorisiert (native `lmd-`-Varianten existieren erst nach Spec #2/#3). Der Port ist eine upstream pressure-getestete Skill — Port-Risiko = Treue + Render-Korrektheit, abgedeckt durch die §5-Gates.

---

## Execution Handoff

**Plan complete and saved to `docs/lean-md/plans/2026-06-29-lmd-test-driven-development-foundation.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — Ich dispatche einen frischen Subagent pro Task, Review zwischen den Tasks, schnelle Iteration. **Beachte:** Bei Subagent-Driven gilt der lean-ctx Multi-Agent-Kontrakt (`.claude/rules/subagent-multi-agent.md`) zwingend — Controller prependet den Dispatch-Contract jedem Subagent-Prompt.

**2. Inline Execution** — Tasks in dieser Session ausführen (executing-plans), Batch mit Checkpoints.

**Welcher Ansatz?**
