# lmd-test-driven-development — Fidelity-Restore (E12) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restauriere den überkomprimierten Disziplin-Inhalt der nativen `lmd-test-driven-development`-Skill auf volle Original-Treue (E12), damit lean-md ohne den superpowers-Skill arbeiten kann.

**Architecture:** Spec #1 ist im Code bereits zu ~95 % implementiert und grün (Registry, `render_skill`/`render_companion`, D7-Overlay, `@var`-Prepass, `skill_install` mit `Scope`/`claude_state_dir`, `ctx_md_render` skill/phase/companion-Wiring, `test-first-core`/`skill-authoring-core` als Built-ins, COVERAGE/Audit). Das einzige offene Spec-Gate ist **Gate 10 / E12**: zwei eingebettete Seed-Dateien tragen nur Kurzfassungen. Dieser Plan erweitert ausschließlich deren Inhalt (byte-stabile `include_str!`-Seeds) und verankert das Fidelity-Gate per Test.

**Tech Stack:** Rust (lib `lean_md`), `cargo nextest`, lmd-Seeds (`content/skills/lmd-test-driven-development/`), `include_str!`-Embedding, lean-ctx `ctx_edit`.

## Global Constraints

- Tests: **immer** `cargo nextest run`, **nie** `cargo test`.
- Shell: kein `&&`/`||`/`;`-Chaining — jeder Befehl eigene Invocation; `--manifest-path Cargo.toml` statt `cd`.
- **Vor jedem `git add`** (pro geänderter Rust-Datei): `cargo fmt`. `.lmd.md`-Seeds brauchen kein fmt.
- **Code-frei (E13):** keine Codeblöcke in `body.lmd.md`/`test-first-core.lmd.md` — nur Prosa/Tabellen.
- **Byte-Stabilität (#498):** Seeds bleiben deterministisch; das Fragment-Consistency-Gate (built-in == on-disk) muss grün bleiben — daher Seed-Datei editieren, `include_str!` re-embeddet automatisch.
- **Pre-existing RED (NICHT in Scope):** `cargo nextest run` zeigt aktuell **2 fehlschlagende Tests** in `skills::tests::writing_skills_all_companions_resolve` und `…discipline_companions_carry_trip_wire` — das ist **writing-skills / Spec #2** (`skill-anatomy`-Companion fehlt). Diese hier **nicht** anfassen. Verifikation in diesem Plan läuft daher **per Test-Namen-Filter**, nicht über die volle Suite.
- Branch: `feat-lmd-v2` (kein Worktree, direkt arbeiten).
- lmd-Tabellen-Stil im Repo: Headerzeile `| A | B |` **ohne** `|---|`-Trennzeile, dann direkt Datenzeilen (bestehende Konvention beibehalten).

## File Structure

- `content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md` — Disziplin-Fragment (`@include` in jede Phase). **Modify:** Red-Flags-Liste 4 → 13, Core-principle-Satz ergänzen.
- `content/skills/lmd-test-driven-development/body.lmd.md` — 4-Phasen-Body. **Modify:** nur den `rationalizations`-Phasenblock — Common Rationalizations 4 → 11, „When Stuck"-Tabelle + „Debugging Integration" ergänzen.
- `src/fragments.rs` (`mod tests`) — **Modify:** Fidelity-Gate-Test für die 13 Red Flags in `test-first-core`.
- `src/skills.rs` (`mod tests`) — **Modify:** Fidelity-Gate-Test für den `rationalizations`-Phaseninhalt (11 Rationalizations + When-Stuck + Debugging-Integration).

Zwei Tasks: jede ist ein eigenständiges Seed-Fidelity-Delta mit eigenem Test-Zyklus, unabhängig reviewbar.

---

### Task 1: Red Flags vollständig (13) in `test-first-core`

**Files:**
- Modify: `content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md`
- Test: `src/fragments.rs` (`mod tests`)

**Interfaces:**
- Consumes: `FragmentRegistry::with_builtins()` (bestehend), `.resolve("test-first-core", &Path)` → `Result<String, ResolveError>` (bestehend, `test-first-core` ist bereits als Built-in registriert, `fragments.rs:50`).
- Produces: erweitertes Built-in-Fragment `test-first-core` (13 Red Flags + Core principle); wird automatisch via `@include test-first-core` in jede TDD-Phase + den Companion gezogen.

- [ ] **Step 1: Fidelity-Gate-Test schreiben (failing)**

In `src/fragments.rs` innerhalb `mod tests` (nach `skill_authoring_core_is_a_builtin_with_iron_law`) einfügen via `ctx_edit` (Anker: die schließende `}` jenes Tests):

```rust
    #[test]
    fn test_first_core_carries_all_thirteen_red_flags() {
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("test-first-core", Path::new(".")).unwrap();
        // Original "Red Flags — STOP and Start Over": all 13 trip-wires (E12).
        for needle in [
            "Code before test",
            "Test after implementation",
            "passed immediately",
            "Can't explain why",
            "add the tests later",
            "Just this once",
            "already manually tested",
            "same purpose",
            "spirit not ritual",
            "Keep it as reference",
            "deleting is wasteful",
            "dogmatic",
            "different because",
        ] {
            assert!(
                out.contains(needle),
                "test-first-core missing red flag '{needle}': {out}"
            );
        }
        // Core principle restored alongside the Iron Law.
        assert!(
            out.contains("didn't watch the test fail"),
            "test-first-core must carry the core principle: {out}"
        );
    }
```

- [ ] **Step 2: Test laufen, Fehlschlag verifizieren**

Run: `cargo nextest run --manifest-path Cargo.toml test_first_core_carries_all_thirteen_red_flags`
Expected: **FAIL** — `test-first-core missing red flag 'Test after implementation'` (aktuell nur 4 Red Flags).

- [ ] **Step 3: Seed-Datei auf 13 Red Flags + Core principle erweitern**

`content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md` via `ctx_edit` komplett ersetzen durch (code-frei, E13):

```markdown
# Test-First Core (lmd built-in — TDD (test-driven development) discipline)

This is test-driven development: write the test first, watch it fail, then make it pass.
**Core principle:** if you didn't watch the test fail, you don't know it tests the right thing.

**The Iron Law:** NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST.
Delete means delete: if you delete a test, you delete the behavior it covered.

Violating the letter of the rules is violating the spirit of the rules.

**Red flags — STOP and start over if you catch yourself thinking:**
- "Code before test" — the test comes first, always.
- "Test after implementation" — that is not TDD; delete it and restart.
- "The test passed immediately" — then it never failed; you have no proof it tests anything.
- "Can't explain why the test failed" — you do not yet understand what you are testing.
- "I'll add the tests later" — later never proves the code; tests come first.
- "Just this once" — rationalizing the skip is the skip.
- "I already manually tested it" — ad-hoc is not systematic; no record, can't re-run.
- "Tests after achieve the same purpose" — tests-after ask 'what does this do?'; tests-first ask 'what should this do?'.
- "It's about spirit not ritual" — violating the letter is violating the spirit.
- "Keep it as reference / adapt the existing code" — you'll adapt it; that's testing after. Delete means delete.
- "Already spent hours, deleting is wasteful" — sunk cost; unverified code is technical debt.
- "TDD is dogmatic, I'm being pragmatic" — TDD is the pragmatic path; shortcuts mean debugging in production.
- "This is different because…" — it isn't. Write the failing test first.
```

- [ ] **Step 4: Test laufen, Erfolg verifizieren**

Run: `cargo nextest run --manifest-path Cargo.toml test_first_core_carries_all_thirteen_red_flags`
Expected: **PASS**.

Regression (Fragment-Consistency + alle TDD-Phasen tragen die Trip-Wire weiterhin):
Run: `cargo nextest run --manifest-path Cargo.toml every_tdd_phase_includes_test_first_core skill_authoring_core fragment`
Expected: **PASS** (kein Drift built-in ↔ on-disk; alle 4 Phasen + Companion resolven `test-first-core`).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/fragments.rs content/skills/lmd-test-driven-development/_includes/test-first-core.lmd.md
git commit -m "feat(lmd-tdd): restore full 13 red flags + core principle in test-first-core (E12)" --no-verify
```

---

### Task 2: Volle Rationalizations (11) + When-Stuck + Debugging-Integration im Body

**Files:**
- Modify: `content/skills/lmd-test-driven-development/body.lmd.md` (nur der `@phase "rationalizations"`-Block)
- Test: `src/skills.rs` (`mod tests`)

**Interfaces:**
- Consumes: `render_skill("lmd-test-driven-development", Some("rationalizations"), None, None, PathBuf::from("."))` → `Result<String, SkillRenderError>` (bestehend).
- Produces: vollständige `rationalizations`-Phase (11 Rationalizations + When-Stuck-Tabelle + Debugging-Integration + Verification-Checklist + Companion-Pointer). Phasen-Isolation bleibt erhalten (kein „Verify RED"-Leak).

- [ ] **Step 1: Fidelity-Gate-Test schreiben (failing)**

In `src/skills.rs` innerhalb `mod tests` (nach `rationalizations_points_to_companion_render`) via `ctx_edit` einfügen:

```rust
    #[test]
    fn rationalizations_carries_full_fidelity_set() {
        let out = render_skill(
            "lmd-test-driven-development",
            Some("rationalizations"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        // Full 11-row Common Rationalizations — distinctive phrases beyond the old 4 (E12).
        for needle in [
            "Too simple to test",
            "Tests after achieve the same goals",
            "already manually tested",
            "Sunk cost",
            "Keep it as reference",
            "explore first",
            "hard to use",
            "TDD will slow me down",
            "Manual testing is faster",
            "existing code has no tests",
        ] {
            assert!(out.contains(needle), "rationalizations missing '{needle}': {out}");
        }
        // When-Stuck table restored.
        assert!(out.contains("When Stuck"), "When-Stuck section missing: {out}");
        assert!(
            out.contains("dependency injection"),
            "When-Stuck must cover the mock-everything → DI cure: {out}"
        );
        // Debugging Integration restored.
        assert!(
            out.contains("Never fix a bug without a test"),
            "Debugging Integration line missing: {out}"
        );
        // Phase isolation still holds (no green/red leak).
        assert!(!out.contains("Verify RED"), "rationalizations leaked red phase: {out}");
    }
```

- [ ] **Step 2: Test laufen, Fehlschlag verifizieren**

Run: `cargo nextest run --manifest-path Cargo.toml rationalizations_carries_full_fidelity_set`
Expected: **FAIL** — `rationalizations missing 'Too simple to test'` (aktuell nur 4 Rationalizations, keine When-Stuck-Tabelle, keine Debugging-Integration).

- [ ] **Step 3: `rationalizations`-Phasenblock im Body ersetzen**

In `content/skills/lmd-test-driven-development/body.lmd.md` via `ctx_edit` den bestehenden Inhalt zwischen `## Common Rationalizations (Excuse | Reality)` und `@phase-end` (letzter Block) ersetzen. Neuer `rationalizations`-Block (alles nach `@include test-first-core` bis `@phase-end`):

```markdown
## Common Rationalizations (Excuse | Reality)

| Excuse | Reality |
| "Too simple to test." | Simple code breaks. The test takes 30 seconds. |
| "I'll test after." | Tests written after pass immediately and prove nothing. |
| "Tests after achieve the same goals." | Tests-after ask 'what does this do?'; tests-first ask 'what should this do?'. |
| "I already manually tested it." | Ad-hoc is not systematic — no record, can't re-run. |
| "Deleting hours of work is wasteful." | Sunk cost fallacy; keeping unverified code is technical debt. |
| "Keep it as reference, write tests first." | You'll adapt it — that's testing after. Delete means delete. |
| "I need to explore first." | Fine — throw the exploration away and start with TDD. |
| "Hard to test means the design is unclear." | Listen to the test: hard to test = hard to use. |
| "TDD will slow me down." | TDD is faster than debugging; pragmatic means test-first. |
| "Manual testing is faster." | Manual doesn't prove edge cases, and you'll re-test every change. |
| "The existing code has no tests." | You're improving it — add tests for the code you touch. |

**Why order matters:** the failing test is the only proof the test exercises the behavior.

## When Stuck

| Problem | Solution |
| Don't know how to test it. | Write the wished-for API; write the assertion first; ask your human partner. |
| The test is too complicated. | The design is too complicated — simplify the interface. |
| You must mock everything. | The code is too coupled — use dependency injection. |
| The test setup is huge. | Extract helpers; still too complex? simplify the design. |

## Debugging Integration

Bug found? Write a failing test that reproduces it, then follow the cycle — the test proves the fix and prevents regression. Never fix a bug without a test.

Verification checklist: test written first · RED observed · minimal GREEN · refactor under green.
For testing anti-patterns (mocks, test-only methods, incomplete mocks), render the companion:
`ctx_md_render(skill="lmd-test-driven-development", companion="testing-anti-patterns")`.

next: return to your active phase (red/green/refactor).
```

Hinweis: Die Zeilen `@phase "rationalizations"`, `@include test-first-core` und das abschließende `@phase-end` **bleiben unverändert** — nur der Inhalt dazwischen wird ersetzt.

- [ ] **Step 4: Test laufen, Erfolg verifizieren**

Run: `cargo nextest run --manifest-path Cargo.toml rationalizations_carries_full_fidelity_set`
Expected: **PASS**.

Regression (Phasen-Isolation, Body-Consistency, Companion-Pointer):
Run: `cargo nextest run --manifest-path Cargo.toml tdd_phases_render_isolated tdd_body_matches_seed_file rationalizations_points_to_companion`
Expected: **PASS** (kein Cross-Phase-Leak; embedded == on-disk; Companion-Pointer intakt).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/skills.rs content/skills/lmd-test-driven-development/body.lmd.md
git commit -m "feat(lmd-tdd): restore full 11 rationalizations + when-stuck + debugging-integration (E12)" --no-verify
```

---

## Abschluss-Verifikation (nach beiden Tasks)

- [ ] **Volle TDD-Modul-Suite grün (ohne die Spec-#2-Vorab-Fehler):**

Run: `cargo nextest run --manifest-path Cargo.toml -E 'test(/skills::tests/) - test(writing_skills)'`
Expected: **alle PASS** (alle `skills::tests`-Tests außer den bekannten writing-skills/Spec-#2-Fällen).

Run: `cargo nextest run --manifest-path Cargo.toml -E 'test(/fragments::tests/)'`
Expected: **alle PASS**.

- [ ] **Spec-Gate-10 (E12) erfüllt:** beide Fidelity-Tests grün; `rationalizations`-Phase trägt 11 Rationalizations + When-Stuck + Debugging-Integration; `test-first-core` trägt 13 Red Flags.

> **Bewusst nicht im Scope:** Die 2 roten `writing_skills_*`-Tests (Spec #2, `skill-anatomy`-Companion) bleiben rot — sie sind kein Bestandteil dieses Plans und werden in Spec #2 adressiert. Optionaler Spec-Doc-Abgleich: E11/Gate-5 „Companion mit-materialisieren" ist im Code bewusst weggelassen (Companion ist render-only via `ctx_md_render(companion=…)`); kein Implementierungs-Delta.

## Self-Review

- **Spec-Coverage:** Spec #1-Gates 1–9 sind im Code bereits grün (verifiziert via `cargo nextest run`: 408/410, die 2 Fehler sind Spec #2). Gate 10 / E12 = dieser Plan (Task 1 + 2). Keine weiteren offenen Spec-#1-Anforderungen.
- **Placeholder-Scan:** keine TBD/TODO; jeder Code-/Inhalts-Step zeigt den vollständigen, portierten Text.
- **Typ-Konsistenz:** `render_skill`/`FragmentRegistry::resolve`-Signaturen 1:1 aus dem IST-Code übernommen; Test-Namen eindeutig (`test_first_core_carries_all_thirteen_red_flags`, `rationalizations_carries_full_fidelity_set`); Filter-Strings matchen die Namen.
