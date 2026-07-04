# lmd-writing-plans — Terseness-Überarbeitung (Spec)

**Datum:** 2026-07-04
**Quelle:** herausgelöst aus `docs/lean-md/specs/2026-07-04-lmd-subagent-driven-development-port-design.md`
(Abschnitt „Eingeschlossen: lmd-writing-plans-Terseness" + „CRP-Mechanik & Bindung")
**Scope:** `content/skills/lmd-writing-plans/body.lmd.md` **+** `content/templates/plan-template.lmd.md`
**+** `content/templates/plan-recipes.lmd.md` **+** `.lean-ctx/lean-md/vars.toml` **+** Tests
**Sprache:** Spec = Deutsch; aller gewobene Content (Body/Template/Recipes) + Code-Kommentare = Englisch

## Ziel

`lmd-writing-plans` soll token-effizientere `.lmd.md`-Pläne erzeugen — **ohne Funktionsverlust**.
Die Ersparnis ist **rein autorenseitig**: weder `consumer` noch `crp` komprimieren authored
Prosa. `apply_crp_hook` (`src/render.rs:287`) hängt nur einen `output_rules`-Guidance-Suffix an;
`Off` = Byte-Passthrough. Der Hebel sind also vier Autoren-Regeln im Skill + eine terse
Recipe-Schicht, nicht eine Engine-Kompression.

Vier Änderungen schließen die offenen `output_rules` (#1 bullets, #2 no-repeat; #3 diffs /
#4 map/signatures sind bereits reflektiert — Anker, `@read mode=diff`):

1. `crp: compact`-Konvention (Header + Skill-Prosa) — der reale Hebel ist die Dispatch-CRP-Zeile.
2. Phase „Bite-Sized" umschreiben: Standard-TDD/Gate/Commit-Zyklus = `@call`-Recipes statt fünf
   ausgeschriebener Prosa-Schritte.
3. Neue Regel „avoid repeating ambient context" (`output_rule` #2).
4. No-Loss-Grenze explizit machen.

## Kernentscheidungen

1. **Scope:** Terseness-Autorenregeln **+** Recipe-Layer in **einer** Spec (aus dem SDD-Port
   herausgelöst, damit writing-plans seinen eigenen Spec→Plan→Impl-Zyklus bekommt). Die
   `@checkpoint`/`@compress`-Bridges bleiben SDD-only (keine writing-plans-Kopplung).
2. **`crp: compact` statt `tdd`** (source-verifiziert, s. u.): autorenseitig sind beide
   byte-identisch (`src/crp.rs`), aber `tdd`s „zero narration" unterdrückt genau das Reasoning
   (Concerns, Design-Begründung, ⚠️-Auflösung), das Reviewer/Controller brauchen; die
   Glyph-Legende amortisiert bei **fresh-per-task** nicht.
3. **Bindung = Plan-Header `crp: compact`** — es gibt keinen eigenen lean-ctx-`crp`-Config-Key.
   Der Header treibt `apply_crp_hook` **und** die Dispatch-Contract-Zeile deterministisch,
   self-contained, byte-stabil (#498), **unabhängig** von der lean-ctx-Config der ausführenden
   Session.
4. **Zwei neue Recipes:** `gate(paths)` (die fehlende Pre-Commit-Qualitätsschranke) und
   `render_check(skill, phase)` (lmd-Render-Smoke). Neue Var `lint_cmd` nach dem Muster von
   `test_cmd` — der echte Wert lebt in `vars.toml`, nicht im Template.
5. **Ambient-Grenze testbar** via Skill-teaches-Tests + Meta-Head-`@include`-Struktur +
   Self-Review-Zeile. **Kein** Heuristik-Content-Lint (fragil, nicht byte-stabil).
6. **Voraussetzung:** Die lmd-Core-Renderer-Bugs **Bug 1 + Bug 3** (s. „Abhängigkeiten &
   Follow-ups") werden **zuerst** gefixt. Diese Spec entwirft die Recipes daher mit natürlichen,
   **quoted** Args (`commit("src/foo.rs", "feat: add foo")`) statt des heutigen
   Entquote-/Kein-Komma-Workarounds.

## Änderungen — Skill-Body (`content/skills/lmd-writing-plans/body.lmd.md`)

**Phase `plan-format` / `no-placeholders` — neue Regel „avoid repeating ambient context":**
Ein Plan wiederholt **keinen** ambienten Kontext, den der Ausführer ohnehin schon trägt:
- Repo-/Build-Plumbing (`include_str!`-Seed-Sync, Manifest-Layout) — höchstens **1×** als
  Architecture-Anker im Meta-Head, nie pro Task.
- Toolset-Rationale, das der Dispatch-Contract nachliefert: Iron-Law-Zitate, Spec-§-Cross-Refs,
  Determinismus-/#498-Reminder.

**No-Loss-Grenze explizit (bindend):** `output_rule` #2 trifft **nur** ambienten Kontext.
**Verbatim** bleiben: **Intent**, **Interfaces/Consumes-Produces**, **NEUER** Code, **Commands**,
**„Expected:"**. Für **existierenden** Code bleibt der Anker die richtige Form (kein Placeholder).

**Phase `bite-sized` umschreiben:** von „fünf Mikroschritte als Absätze" (`Write the failing
test` / `Run it to make sure it fails` / `Implement…` / `Run the tests…` / `Commit`) →
**„Standard-TDD/Gate/Commit-Zyklus = `@call`-Recipes; nur nicht-Boilerplate-Schritte
ausgeschrieben."** Das löst den bestehenden Widerspruch zwischen der Prosa-Phase und der
Recipe-Schicht (`tdd`/`gate`/`commit` existieren bereits als Makros).

**Phase `plan-format` — `crp: compact`-Konvention nennen:** der `plan-format`-Text weist an,
dass der `plan-template`-Header `crp: compact` (neben `consumer: ai`) trägt, und benennt die
„avoid repeating ambient context"-Regel als `output_rule` #2.

## Änderungen — Template (`content/templates/plan-template.lmd.md`)

- **Header:** `+ crp: compact` (neben `consumer: ai`; bisher fehlte jede `crp:`-Zeile → Default
  `off`).
- **Meta-head:** `+ @var lint_cmd default="cargo clippy --all-targets -- -D warnings"
  desc="project lint gate"` (Muster wie `test_cmd`; `vars.toml` gewinnt).
- **„Verify & Close"-Sequenz** (feste Reihenfolge) wird auf den Gate umgestellt:

      @call verify("src/foo.rs")
      @call gate("src/foo.rs")
      @call commit("src/foo.rs", "feat: add foo")
      @call remember_decision("foo is now the canonical helper fn")

  `gate` trägt `@reformat` + `lint_cmd` + `test_cmd` (Voll-Suite) und ersetzt damit
  `reformat_commit` **im Standard-Zyklus**; `commit` wird das schlanke Plain-Commit (fmt schon
  im Gate erledigt). `reformat_commit` **bleibt** im Recipe-Set für leichte Tasks ohne vollen
  Gate. Die **Conditional-Slots** (`recall_context` bei Folge-Task, `callers` + `@refactor` bei
  Symbol-Umbau, `review_change` bei Public-API/Multi-Datei, `inspect` bei IDE-Quality-Pass)
  bleiben unverändert; **neuer Slot:** bei Tasks, die einen Skill-/Plan-`.lmd.md`-Seed ändern →
  `@call render_check("<skill>", "<phase>")` (Render-Smoke des betroffenen Phasen-Renders). Damit
  ist `render_check` an ein beobachtbares Prädikat gebunden statt Orphan-Recipe.

## Änderungen — Recipes (`content/templates/plan-recipes.lmd.md`)

Zwei neue `@define` (jeweils mit HTML-Kommentar-Erstzeile — Index-Vollständigkeits-Gate):

    @define gate(paths)
    <!-- Pre-commit quality bar: reformat, lint, full test suite (lint_cmd/test_cmd via vars.toml) -->
    1. Run: `@reformat {{ paths }}`
    2. Run: {{ var lint_cmd }} — Expected: clean.
    3. Run: {{ var test_cmd }} — Expected: PASS.
    @define-end

    @define render_check(skill, phase)
    <!-- Render one skill/plan phase via the CLI; assert non-empty + byte-stable (#498) -->
    Run: cargo run -q --bin lean-md -- render --skill {{ skill }} --phase {{ phase }} --consumer=ai
    — Expected: non-empty, no eval err, byte-stable across two runs.
    @define-end

`gate` bleibt **sprach-agnostisch**; die Rust-Spezifika (nextest, clippy-Flags) sind
ausschließlich `vars.toml`-Override-Werte. Kein Monolith-Recipe `task_cycle(...)` — Verify&Close
bleibt bewusst getrennte Calls (verschiedene Args, Conditional-Slots dazwischen). **Kein**
`red_dispatch`-Recipe (Dispatch = SDD-/Executor-Territorium, nicht Plan-Inhalt).

## Änderungen — `.lean-ctx/lean-md/vars.toml`

    lint_cmd = "cargo clippy --all-targets -- -D warnings"

(Analog zu `test_cmd = "cargo nextest run"`; `vars.toml` > Seed-Default.)

## CRP-Mechanik & Bindung (source-verifiziert)

Zwei getrennte CRP-Oberflächen (Details in der SDD-Spec):

- **Render-CRP** (`apply_crp_hook`, `src/render.rs:287`): hängt bei `compact`/`tdd` einen
  `output_rules`-Suffix an. `crp_output_rules(Compact) == crp_output_rules(Tdd)` byte-identisch
  (`src/crp.rs`); **nur** `tdd` zusätzlich eine Glyph-Legende. → **autorenseitig `compact == tdd`.**
- **Dispatch-CRP** (`src/bridges/dispatch.rs`, realer Hebel): substituiert `{{ crp }}` im
  `dispatch-contract`-Seed → Contract-Zeile „render in CRP mode `compact`". Einziger Ort, an dem
  `compact` vs. `tdd` real divergiert.

**Warum `compact`:** `compact` = „omit filler; abbreviate …; ≤200 tok; trust tool outputs".
`tdd` = „max density; ≤150 tok; **zero narration**" + Legende (~60 tok Fixkosten). `tdd`s
„zero narration" unterdrückt das Reasoning, das task-reviewer/code-reviewer/Controller zum
Bewerten brauchen; die Legende amortisiert bei fresh-per-task nicht.

## Ambient-Grenze (testbar)

- **Struktur (der eigentliche Gate):** Ambient-Kontext hat ein kanonisches Einmal-Zuhause — den
  Meta-Head (Global Constraints, `@include`-referenceable; „every task implicitly includes it").
  Tasks `@include`n, statt zu restaten. Dieser Mechanismus existiert bereits; die neue Regel
  benennt ihn nur.
- **Assert-Test:** `writing_plans_teaches_crp_compact_and_no_repeat` (Body nennt `crp:compact` +
  die Regel) — Muster wie `writing_plans_body_weaves_code_intel`.
- **Self-Review-Zeile:** „Ambient erscheint einmal im Meta-Head, nicht pro Task wiederholt."
- **Bewusst NICHT:** ein Heuristik-Lint über generierte Pläne (Rationale-Duplikat-Erkennung) —
  fragil, nicht byte-stabil, verletzt #498-Ethos. YAGNI.

## Sicherheits-Kopplung

Das Weglassen ambienten Kontexts ist sicher, weil SDDs Dispatch-Contract
(`content/core/dispatch-contract.lmd.md`) Hard-Rules + Tool-Discipline nachliefert und
`render --phase task-N` den Task self-contained macht.
**Zukunfts-Constraint (in der Body-Guidance verankern):** sobald `lmd-executing-plans` portiert
wird, MUSS es dieselbe Baseline prependen — sonst leakt die Auslassung bei Inline-Ausführung.

## Tests & Gates

Skill (`src/skills.rs`):
- **`writing_plans_teaches_crp_compact_and_no_repeat`** — `plan-format` nennt `crp: compact` +
  die „avoid repeating ambient context"-Regel.
- `writing_plans_body_weaves_code_intel`, Phasen-Isolation, Byte-Stabilität bleiben grün.

Template/Recipes (`src/seeds.rs`):
- **`plan_template_header_declares_crp_compact`** — Header trägt `crp: compact` neben
  `consumer: ai`.
- **`plan_template_meta_declares_lint_cmd`** — `@var lint_cmd` im Meta-Head.
- **`plan_recipes_carry_gate_and_render_check`** — beide `@define` präsent.
- `plan_recipes_all_documented` / `no_orphan_call` bleiben grün (Index-Vollständigkeit; `gate`
  und `render_check` tragen die HTML-Kommentar-Erstzeile).

Behavioral (Plan-Task-Ebene, kein `cargo`-Gate):
- **Preflight-Ordering-Gate (abgeleiteter Plan, Task 1, verpflichtend):** Der Plan MUSS als
  erste Task verifizieren, dass Bug-1s quote-bewusster Split **gelandet** ist — Repro
  `@call commit("a", "b, c")` rendern und asserten, dass das zweite Arg „b, c" (mit Binnen-Komma,
  **ohne** Quote-Leak) überlebt. Erst nach bestandenem Gate dürfen die Template-/Recipe-Tasks
  quoted, kommahaltige Args einführen. Das macht die Reihenfolge-Abhängigkeit zu einem harten
  Gate statt bloßer Prosa (sonst bricht der quoted Template-Beispiel-Arg die
  Fragment-Konsistenz-/Byte-Stabilitäts-Gates).
- Render-Smoke pro Body-Phase via CLI, byte-stabil.
- Die quoted, kommahaltigen Recipe-Args (`commit(...)`, `remember_decision(...)`) expandieren
  korrekt — der eigentliche Bug-1-Beleg; `gate`/`render_check` rendern non-empty + byte-stabil.

## Abhängigkeiten & Follow-ups (out of scope, getrackt)

Entdeckt beim GREEN-Pressure-Test der Bridge-Bindung (Ledger `lmd-core-followups-from-green`):

- **Prerequisite — Bug 1 (KRITISCH):** Der `@call`-Argument-Scanner splittet auf **jedem**
  literalen Komma — auch innerhalb `"…"` — und strippt Quotes nicht. Bei Ein-Arg-Recipes
  (`remember_decision(content)`) wird alles nach dem ersten Komma **still** verschluckt;
  `commit("src/foo.rs", "feat: add foo")` → `git commit -m ""feat: add foo""` (Quote-Leak).
  Repro: `@define note(c) … @define-end` + `@call note(hello, world)` → erwartet `{{ c }}` =
  „hello, world", ist „hello". **Fix** = quote-bewusster Split (Kommas in `"…"` ignorieren) +
  Quote-Stripping, **ohne** die legitimen Mehr-Arg-Recipes (`reformat_commit(paths, msg)`,
  `commit(paths, msg)`) zu brechen (deren Top-Level-Komma ist der legitime Separator).
  Kandidaten: `src/bridges/call.rs`, `src/macros.rs`. Eigener systematic-debugging + TDD-Zyklus,
  **separater Commit**; #498 + bestehende `macros`/`bridges::call`-Tests grün. **Diese Spec setzt
  den Fix voraus.**
- **Prerequisite — Bug 3:** `render <plan>.lmd.md` **ohne** `--phase` lässt
  `@import .lean-ctx/lean-md/plan-recipes` mit NotFound fehlschlagen → jeder `@call` kaskadiert
  mit „macro not found". Mit `--phase task-N` funktioniert derselbe Import. Erklärt zugleich die
  `ctx_read`-`.lmd.md`-Unzuverlässigkeit (interner Whole-Document-Pfad). **Fix** = Whole-Doc-Render
  löst `@import` **identisch** zum `--phase`-Pfad auf (Import-Basis: `jail_root`/working-dir).
  Kandidaten: `src/engine.rs`, `src/phases.rs`, `@import`-Bridge. Separater Commit.
- **Follow-up — Directive-Grammatik (eigenes Audit-Spec):** `gloss/directives.lmd.md`
  `graph:callers|callees|dependents` nutzen **benannte** Slots `{callers}`/`{callees}`/
  `{dependents}`, während die Call-Form positional ist (`@graph callers <symbol>`) → Slot bleibt
  unbound, sollte `{1}` sein (analog `symbol:*`). Zusätzlich: `@symbol`/`@impact`/`@refactor` sind
  bare dokumentiert, real brauchen sie Op-Keywords + Named-Args (`@symbol overview <n>`,
  `@impact analyze <s>`, `@refactor rename path= line= new=`) → Plan-Autor landet in
  Trial-and-Error.
- **Kein Bug (bestätigtes Design):** `@call`-Recipes expandieren zu Prosa (`Run: @directive`); nur
  bare, zeilenführende Directives werden live dispatcht (#498).
- **Verifiziert-real, kein Concern:** der `@query git diff | @review diff-review`-Pipe
  (`ReviewBridge::accepts_pipe() == true`, `src/bridges/review.rs:28`, Test `:170`).

## Explizite Nicht-Ziele (YAGNI)

- Kein Monolith-Recipe `task_cycle(...)` — Verify&Close bleibt getrennte Calls.
- Kein Heuristik-Content-Lint für die Ambient-Grenze.
- Keine Änderung an `apply_crp_hook` / der CRP-Engine (der Plan-Header genügt).
- Kein `red_dispatch`-Recipe (SDD-Territorium).
- `test_cmd`/`lint_cmd`-Werte werden **nicht** ins Template hartkodiert (`vars.toml` gewinnt).
- Die Renderer-Bugfixes (Bug 1/3) sind **kein** Skill-Content — sie sind Prerequisite, nicht
  Task dieser Spec.
