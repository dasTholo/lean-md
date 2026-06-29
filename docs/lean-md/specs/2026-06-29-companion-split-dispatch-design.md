# Companion-Split + Dispatch-Integration für lmd-writing-skills (Design)

> **Status:** Design (brainstormed, freigegeben). Nächster Schritt: `writing-plans`.
> **Vorgänger:** `docs/lean-md/specs/2026-06-29-lmd-writing-skills-port-design.md`
> (Voll-Fidelity-Port). Dieser Spec **erweitert** den Port und **ersetzt** dessen
> offenes Task 8 (Full-Gate) durch den finalen Full-Gate dieses Plans.

## Goal

Den einzigen wirklich großen Companion `testing-skills-with-subagents` (~390 Z.)
in **drei** granular abrufbare On-Demand-Einheiten splitten, eine leichte
hierarchische `group/name`-Namens-/Ordnerkonvention einführen, und die
`@dispatch`-Bridge so erweitern, dass die `writing-skills`-Body-Phase einen
**Tester-Subagenten** mit der Test-Methodik als Brief dispatchen kann.

Die anderen 7 Companions bleiben **unverändert** (YAGNI: je eine kohärente
Referenz in vernünftiger Größe; ein Split zerrisse zusammengehörige Prosa für
minimale Token-Ersparnis).

## Architecture

Zwei unabhängige, aber zusammen ausgelieferte Änderungen:

1. **Companion-Split (kein Engine-Change).** `companion_body`/`render_companion`
   matchen den Companion-Key als freien String — `"testing/methodology"` ist nur
   ein Key, Seeds bleiben `include_str!`-embedded (byte-stabil #498). Der Split
   ist damit reine Seed- + Registry- + Stub-Arbeit. Der `/`-Pfad-Teil spiegelt
   sich nur auf der Platte (`companions/testing/*.lmd.md`) zur Gruppierung; es
   gibt keine Laufzeit-Pfadauflösung und keine PathJail-Implikation.

2. **Dispatch-Integration (Engine-Change, auf `bridges/dispatch.rs` begrenzt).**
   Die Bridge bezieht ihren Brief heute ausschließlich aus `ctx.phase_body(phase)`.
   Sie wird um eine zweite Brief-Quelle erweitert: `skill=`+`companion=` →
   `crate::skills::companion_body(skill, companion)`. Das Skill wird **explizit
   in der Direktive** mitgegeben (statt durch den `EngineContext` gefädelt), damit
   die Änderung auf `bridges/dispatch.rs` (+ Rollenvalidierung + Tests) beschränkt
   bleibt — kein `engine.rs`-Eingriff.

## Tech Stack

Rust (standalone crate `lean_md`, lib + bin), `cargo nextest`, lean-ctx
MCP-Tooling. Keine neuen Dependencies.

## Global Constraints

- **Tests:** immer `cargo nextest run`, nie `cargo test`. Crate ist standalone;
  Kommandos laufen aus dem Repo-Root (kein `cd`, kein `--manifest-path`).
- **Shell:** kein `&&`/`||`/`;`-Chaining — jedes Kommando eine eigene Invocation.
- **Vor jedem `git add`** (je geänderte Code-Datei): `cargo fmt`.
- **No worktrees** — direkt auf `feat-lmd-v2`.
- **Determinismus (#498):** Tool-Output ist deterministische Funktion von
  (Inhalt, Mode, CRP, Task); embedded Seeds byte-identisch zur on-disk-Quelle
  (Fragment-/Companion-Consistency-Gates müssen grün bleiben); `CliBackend` ==
  `McpBackend`. Keine Timestamps/Counter/Random.
- **Fidelity (kein Verlust):** die Summe der 3 neuen `testing/*`-Companions ist
  inhaltlich identisch zum bisherigen `testing-skills-with-subagents` (verbatim;
  nur die neuen Header-Kommentarzeilen + dokumentierte Reference-Closure-Edits
  unterscheiden sich). Keine Original-Sektion geht verloren.
- **Reference-Closure:** alle Querverweise zeigen auf lmd-/lean-md-Ziele, nie
  zurück nach superpowers; Querverweise zwischen den 3 neuen Companions werden zu
  `ctx_md_render(... companion="testing/<name>")`.
- **Naming (E2):** Acronym „TDD" nur disambiguiert („TDD (test-driven
  development)"), nie nackt als Keyword. Bestehende Disambiguierungen bleiben.
- **Code/Kommentare:** Englisch. Interaktion/Commits-Prosa: Deutsch mit Umlauten.
- **Tool-Discipline:** lean-ctx MCP-Tools statt nativ; `.lmd.md`-Rohquelle nur
  über `git show HEAD:<path>` lesbar (Shadow-Hook rendert sonst).

---

## File Structure

**Neue Seed-Dateien (alle embedded via `include_str!`):**

- `content/skills/lmd-writing-skills/companions/testing/methodology.lmd.md`
- `content/skills/lmd-writing-skills/companions/testing/skill-types.lmd.md`
- `content/skills/lmd-writing-skills/companions/testing/creation-checklist.lmd.md`

**Entfernte Seed-Datei:**

- `content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md`

**Geänderte Dateien:**

- `content/skills/lmd-writing-skills/body.lmd.md` — green-Phase erhält
  `@dispatch skill=… companion="testing/methodology" role=test …`; refactor-Phase
  verweist auf Re-Dispatch.
- `content/skills/lmd-writing-skills/SKILL.md` — Companion-Liste: 1 → 3 Einträge.
- `src/skills.rs` — Consts: `LMD_WS_TESTING_SUBAGENTS` → drei
  (`LMD_WS_TESTING_METHODOLOGY`, `LMD_WS_TESTING_SKILL_TYPES`,
  `LMD_WS_TESTING_CREATION_CHECKLIST`); `COMPANIONS`-Zeile 1 → 3; Tests anpassen.
- `src/bin/lean_md.rs` — CLI==MCP-Companion-Gate auf neuen Namen.
- `src/bridges/dispatch.rs` — `companion=`-Brief-Quelle + `test`-Rolle + Tests.
- `src/availability.rs` — `COVERAGE`: `testing-skills-with-subagents` →
  `testing/methodology`; neue `dispatch`-Zeile; Test ggf. anpassen.
- `content/tooling/availability-audit.md` — Abschnitt aktualisieren.

---

## Detail 1 — Companion-Split

**Schnittgrenzen** (aus dem bestehenden `testing-skills-with-subagents.lmd.md`,
verifiziert via `git grep -n '^#'`):

| Neue Datei | Quell-Bereich (alte Datei) | Header-Kommentarzeile |
|---|---|---|
| `testing/methodology.lmd.md` | Z.1–388: Header-Kommentar + `@include skill-authoring-core` + `# Testing Skills With Subagents` … bis inkl. `## Real-World Impact` | `# Testing Methodology (lmd companion — RED→GREEN→REFACTOR testing workflow for skills)` |
| `testing/skill-types.lmd.md` | `## Testing All Skill Types` (+ 4 Subsektionen Discipline/Technique/Pattern/Reference) | `# Testing Skill Types (lmd companion — how to test discipline/technique/pattern/reference skills)` |
| `testing/creation-checklist.lmd.md` | `## Skill Creation Checklist (TDD Adapted)` bis EOF | `# Skill Creation Checklist (lmd companion — TDD-adapted checklist before deploying a skill)` |

**Regeln:**

- `testing/methodology` **behält** `@include skill-authoring-core` am Kopf (es ist
  die Disziplin-Erzählung → trägt den Iron-Law-Marker
  `NO SKILL WITHOUT A FAILING TEST FIRST`).
- `testing/skill-types` und `testing/creation-checklist` sind reine Referenz/
  Checkliste → **kein** `@include` (lean halten; Disziplin lebt in `methodology`).
- Jede neue Datei startet mit der obigen Header-Kommentarzeile, dann verbatim der
  Quell-Bereich (die bisherige `# Testing Skills With Subagents`-Zeile bleibt nur
  in `methodology`).
- **Reference-Closure:** falls eine der 3 auf eine andere verweist → `render the
  companion: ctx_md_render(skill="lmd-writing-skills", companion="testing/<name>")`.
  (Die bestehende, bereits geschlossene `persuasion-principles`-Referenz in der
  Methodik bleibt unverändert.)

**Registry (`src/skills.rs`):** die Zeile
`("lmd-writing-skills", "testing-skills-with-subagents", LMD_WS_TESTING_SUBAGENTS)`
wird ersetzt durch:

```rust
("lmd-writing-skills", "testing/methodology", LMD_WS_TESTING_METHODOLOGY),
("lmd-writing-skills", "testing/skill-types", LMD_WS_TESTING_SKILL_TYPES),
("lmd-writing-skills", "testing/creation-checklist", LMD_WS_TESTING_CREATION_CHECKLIST),
```

mit drei `include_str!`-Consts auf die `companions/testing/*.lmd.md`-Pfade.
`companion_body`/`render_companion` bleiben unverändert.

---

## Detail 2 — SKILL.md-Stub / COVERAGE / Audit

- **`SKILL.md`-Stub:** der Eintrag
  `testing-skills-with-subagents — full testing methodology + creation checklist`
  → drei Einträge:
  - `testing/methodology — RED→GREEN→REFACTOR testing workflow (pressure scenarios, rationalization tables)`
  - `testing/skill-types — how to test discipline/technique/pattern/reference skills`
  - `testing/creation-checklist — TDD-adapted checklist before deploying a skill`

  Render-Call-Beispiel auf `companion="testing/methodology"` aktualisieren.
- **`availability.rs` `COVERAGE`:** Zeile
  `("lmd-writing-skills", "testing-skills-with-subagents", "include", "fragment-compose")`
  → `("lmd-writing-skills", "testing/methodology", "include", "fragment-compose")`;
  zusätzlich **neue** Zeile für die Dispatch-Nutzung:
  `("lmd-writing-skills", "green", "dispatch", "fragment-compose")`
  (Direktive `dispatch` ist in `default_registry()` registriert →
  `every_covered_directive_is_registered` bleibt grün).
- **`content/tooling/availability-audit.md`:** Coverage-Abschnitt entsprechend
  aktualisieren (Companion-Zeile umbenennen, Dispatch-Zeile ergänzen).

---

## Detail 3 — Dispatch-Engine-Change (`src/bridges/dispatch.rs`)

**Brief-Quelle (neu):** genau eine von
- `phase=<name>` (bestehend) → `ctx.phase_body(phase)`, **oder**
- `skill=<skill>` + `companion=<companion>` (neu) →
  `crate::skills::companion_body(skill, companion)`.

Verhalten:
- Beide Quellen gegeben → `BridgeError::Resolve` („use exactly one of phase= or
  companion=").
- Keine Quelle → `MissingArg` (wie bisher bei fehlendem `phase`).
- `companion=` ohne `skill=` → `MissingArg("skill")`.
- Companion nicht gefunden → `<!-- lmd: COMPANION_NOT_FOUND '<skill>/<companion>' -->`
  Envelope (analog zum bestehenden `PHASE_NOT_FOUND`-Envelope; **kein** Abbruch).
- Companion-Body wird **work-lazy** gerendert (konsistent zum Phasen-Pfad:
  Templates/`{{ }}` aufgelöst, `@include skill-authoring-core` inline, Work-
  Direktiven wie `@read` verbatim für den Subagenten). Komposition unverändert:
  Contract (b) + Brief (a) unter `## Task (phase-isolated)` + Bootstrap (c).

**Rolle `test` (neu):** Validierung `Some(r @ ("dev" | "review" | "test"))`;
`{{ role }}` → `test`. (Die `dispatch-contract`-Seed nutzt `role={{ role }}`
generisch — keine Seed-Änderung nötig.)

Alle bestehenden Guards bleiben: `to_agent`-Sentinel/M-2-Template-Injection-Guard,
`{{ controller_id }}`-Platzhalter-Erhalt bei fehlendem `to_agent`, NUL-freie
Ausgabe (M-3), CRP-Threading.

---

## Detail 4 — Body-Verdrahtung (`body.lmd.md`)

- **green-Phase:** nach der bestehenden GREEN-Prosa ein Dispatch-Block:

  ```
  To pressure-test the skill you just wrote, dispatch a tester subagent whose
  brief is the full testing methodology:

  @dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test to_agent="{{ controller_id }}"
  ```

  Beim Rendern der green-Phase materialisiert `@dispatch` den vollständigen
  Tester-Prompt (Contract + Methodik inkl. Iron Law + Bootstrap). `to_agent` ist
  der ausfüllbare Platzhalter `{{ controller_id }}` (M-2-Guard injiziert ihn
  literal nach `render_body`; er erscheint als `to_agent={{ controller_id }}`).
- **refactor-Phase:** Prosa-Hinweis „after closing loopholes, re-dispatch the same
  tester (`@dispatch … companion="testing/methodology" role=test`) and re-verify"
  — **nicht** materialisiert (vermeidet doppelte, große Dispatch-Blöcke; die
  green-Phase trägt die kanonische, ausführbare Direktive).
- Phasen-Isolation bleibt: green/refactor werden weiterhin isoliert gerendert;
  der `@dispatch` lebt im green-Phasen-Body und leakt nicht in andere Phasen.

---

## Detail 5 — Tests

**Split / Companion-Registry:**
- `writing_skills_all_companions_resolve` — Namensliste auf die 3 `testing/*` +
  die 7 unveränderten Companions umstellen; je `companion_body(...)` `Some` +
  non-empty.
- `writing_skills_discipline_companions_carry_trip_wire` — Loop über
  `["testing/methodology", "bulletproofing"]`; beide rendern mit Iron-Law-Marker;
  Guard `!out.contains("writing-skills directory")` bleibt.
- `writing_skills_testing_companion_carries_skill_md_sections` — umstellen:
  `render_companion("…","testing/skill-types",…)` enthält „Testing All Skill
  Types"; `render_companion("…","testing/creation-checklist",…)` enthält „Skill
  Creation Checklist (TDD Adapted)".
- `writing_skills_fidelity_all_surfaces_render_nonempty` — Companion-Namensliste
  auf die 3 `testing/*` umstellen (alle weiterhin `> 80` Zeichen).
- CLI==MCP-Gate (`src/bin/lean_md.rs`) — Companion-Name auf `testing/methodology`.
- **Neuer Fidelity-Test:** `testing/methodology` enthält einen Methodik-Marker
  (z.B. „RED Phase: Baseline Testing"); die 3 zusammen tragen alle ursprünglichen
  Top-Level-Sektionen (Stichproben-Marker je Datei).

**Dispatch-Bridge (`src/bridges/dispatch.rs`):**
- `dispatch_companion_brief_composes_contract_methodology_bootstrap` — ein Doc mit
  `@dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test
  to_agent="c"` rendert: „Subagent Contract", `role=test`, ein Methodik-Marker
  („RED Phase"), Iron Law (`NO SKILL WITHOUT A FAILING TEST FIRST`, via @include),
  Bootstrap.
- `dispatch_phase_source_still_works` — bestehender `phase=`-Pfad unverändert grün.
- `dispatch_rejects_both_phase_and_companion` — both-given → Resolve-Fehler.
- `dispatch_companion_requires_skill` — `companion=` ohne `skill=` → MissingArg.
- `dispatch_unknown_companion_yields_envelope` — `COMPANION_NOT_FOUND`-Envelope,
  kein Abbruch.
- `test`-Rolle: `dispatch_test_role_substitutes`.

**Body:**
- `green_phase_renders_tester_dispatch_block` — `render_skill("lmd-writing-skills",
  Some("green"),…)` enthält den Dispatch-Block (Contract + Methodik-Marker +
  Bootstrap + `to_agent={{ controller_id }}`); refactor leakt nicht in green.

**Determinismus (#498):** je ein Byte-Stabilitäts-/CLI==MCP-Sample für die neuen
Companion- und Dispatch-Surfaces.

---

## Fidelity-Coverage-Matrix (Split)

| Original-Sektion (alte Datei) | Neues Ziel | Status |
|---|---|---|
| Header + `@include` + Overview … Real-World Impact (L1–388) | `testing/methodology` | verbatim (Header-Kommentar neu) |
| `## Testing All Skill Types` (+4 Subs) | `testing/skill-types` | verbatim |
| `## Skill Creation Checklist (TDD Adapted)` | `testing/creation-checklist` | verbatim |

Kein Original-Inhalt ohne Ziel; kein Ziel ohne Original (außer den 3 neuen
Header-Kommentarzeilen).

---

## Sequencing

Dieser Umbau ändert Companions + Body + Engine und **ersetzt** das offene Task 8
(Full-Gate) des Port-Plans `2026-06-29-lmd-writing-skills-port`. Der Full-Gate
(fmt-check, komplette Suite, clippy `-D warnings`, Determinismus, Render-Smoke)
läuft als **finaler Task dieses Plans**. Port-Commits T1–T7 (bis `ea58cb4`)
bleiben unverändert. Arbeit auf `feat-lmd-v2`, TDD-Tasks via
subagent-driven-development.

## Self-Review (Scope/Placeholder/Konsistenz)

- **Scope:** ein fokussierter Plan — genau ein Companion gesplittet + eine
  begrenzte Engine-Erweiterung. Kein Reorg der übrigen 7 Companions.
- **Placeholder:** keine TBD/TODO; alle Schnittgrenzen, Registry-Zeilen,
  Bridge-Regeln und Tests konkret benannt.
- **Konsistenz:** Companion-Namen identisch in Seed-Pfaden, Registry, Stub,
  COVERAGE, Tests, Body-`@dispatch`. `testing/methodology` ist die einzige der 3
  mit `@include skill-authoring-core` (Trip-Wire) — konsistent in Detail 1 + Test
  `writing_skills_discipline_companions_carry_trip_wire`.
- **Ambiguität:** Dispatch-Brief-Quelle ist exklusiv (phase XOR skill+companion);
  `test`-Rolle additiv; refactor referenziert statt materialisiert (explizit).
