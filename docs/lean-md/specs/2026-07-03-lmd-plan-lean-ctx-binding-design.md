# lmd-writing-plans — Pläne stringenter an lean-md/lean-ctx binden (Design)

**Datum:** 2026-07-03
**Status:** Entwurf zur Review
**Kontext:** `lmd-writing-plans` ist portiert (7-Phasen-Body + `plan-recipes`/`plan-template`-Seeds + `plan-reviewer`-Companion + Index-Gates). Beobachtung: Die generierten `.lmd.md`-Pläne nutzen nur einen Bruchteil der vorhandenen lean-md-Bridges (und damit der lean-ctx-Brücken). Ziel: die Pläne **real** und **durchgesetzt** an die Bridge-Fläche binden — token-sicher unter der in §6a der Port-Spec etablierten Makro-Lib-Skalierung.

---

## 1. Problem & Ist-Zustand

lean-md registriert 29 Directive-Bridges (`default_registry()`), darunter **alle** vom Nutzer gewünschten: `@review @graph @impact @find @smells @recall` (ctx_knowledge recall) `@remember` (ctx_knowledge write) `@handoff @dispatch` + Session-Sinks. Genutzt wird davon in generierten Plänen fast nichts:

- `plan-recipes` Makro-Lib = nur `{test, commit, tdd}` (reines TDD-Boilerplate).
- `plan-template` demonstriert nur `@read`/`@symbol`/`@call`.
- `writing-plans`-Body erwähnt Anker + `@read mode=diff` — kein `@graph/@impact/@review/@smells/@find/@recall`.
- `availability.rs` COVERAGE für `lmd-writing-plans` ist dünn: nur `read/list/edit/remember/dispatch`.

Die Bindung existiert also als Fähigkeit, aber nicht als Vorgabe. Sie ist weder demonstriert (Template) noch erzwungen (Gate) — deshalb ignoriert das Plan-schreibende LLM sie.

## 2. Scope-Grenze (gesetzt)

| Ebene | Skill | Bridges |
|---|---|---|
| **Autoren** (Plan-Autor beim Schreiben) | `lmd-writing-plans` **jetzt** | `@recall @graph @impact @find` (+ `@remember` bestehend) |
| **Plan-Inhalt** (in Plan eingebettet, expandiert für Executor) | `lmd-writing-plans` **jetzt** | `@review @smells @inspect @reformat @remember @recall @graph` (als `@call`-Recipes) |
| **Koordination** | `lmd-subagent-driven-development` **später** | `@dispatch @handoff` + `ctx_agent/ctx_session/ctx_task` |

Die Koordinations-Bridges leben bereits im `dispatch-contract`-Seed (Contract-Kanon, D-7) — **nicht duplizieren**; sie gehören zum Ausführungs-, nicht zum Plan-Schreib-Kontext.

## 3. Ansatz: Layered Weave + COVERAGE-Gate

Vier Flächen, jede in ihrer Rolle — der einzige Ansatz, der DRY *und* durchgesetzt ist:

1. **`plan-recipes`** += neue `@define`-Makros (Plan-Inhalt, expandiert für den Executor).
2. **`writing-plans`-Body** += „Authoring mit Code-Intel"-Block (Autoren-Directives) + Verifikations-Prosa ruft die neuen Recipes.
3. **`plan-template`** demonstriert die Kern-Recipes per `@call` (sonst ignoriert das Autor-LLM ungenutzte Makros).
4. **`availability.rs` COVERAGE** += Zeilen → der Fidelity-Gate `every_covered_directive_is_registered` beweist die Verdrahtung.

Verworfen: **Recipes-only** (keine Durchsetzung → Bindung verrottet) und **Body-only** (nicht DRY → Pläne blähen auf, jeder buchstabiert dieselben Directives neu).

## 4. Neue `plan-recipes`-Makros

Alle folgen dem etablierten „**Run:**"-Muster der bestehenden Recipes: der Body emittiert **Instruktionen** mit inline-`code`-Directives (nie zeilenführend → guidance, nicht render-aktiv), die der ausführende Subagent zur Task-Zeit ausführt. Kein Execute-at-Render → #498-Determinismus gewahrt (kein `git diff`/Backend-Call im Output-Body).

| Recipe | Backing (lean-ctx) | Rolle |
|---|---|---|
| `verify(paths)` | `@read mode=diff` → ctx_read | Change per Diff inspizieren statt Copy-Paste |
| `review_change()` | `@query git diff \| @review diff-review` → ctx_review | Review-Gate: fusioniert impact + caller-tracking + smells + test-discovery |
| `check_smells(path)` | `@smells` → ctx_smells | Smell-Gate (Default `scan`, surfacet Findings) |
| `inspect(path)` | `@inspect` → ctx_refactor inspections, **Fallback** `@smells` | IDE-Diagnostics falls Backend live, sonst headless Smell-Scan |
| `reformat_commit(paths,msg)` | `@reformat` + commit → ctx_refactor reformat | Pre-Commit-Format integriert in den Commit-Schritt |
| `remember_decision(content)` | `@remember` → ctx_knowledge | durable Fakt/Gotcha am Task-Ende |
| `recall_context(query)` | `@recall` → ctx_knowledge | durable Kontext am Task-Start ziehen |
| `callers(symbol)` | `@graph callers` → ctx_callgraph | „wer ruft das auf" — Anker für Refactor-Tasks |

### 4a. `inspect(path)` — Backend-Priorität mit Fallback

Der Nutzer-Wunsch: das Makro prüft, ob das (IDE-)Backend verfügbar ist — wenn ja priorisieren, sonst Alternative. **Architektur-Randbedingung:** `@inspect` liefert headless ein `BACKEND_REQUIRED`-Envelope; eine Verzweigung des **Render-Outputs** auf Live-Backend-Verfügbarkeit bräche #498 (Determinismus = Funktion von Dateiinhalt/Mode/Task, nicht Umgebung). Deshalb ist die Verzweigung eine **Executor-Instruktion**, keine Render-Zeit-Bedingung — das Makro expandiert deterministisch, der Agent prüft zur Task-Zeit:

```
@define inspect(path)
<!-- IDE inspections if an IDE backend is live, else a headless smell scan -->
1. Run: `@inspect {{ path }}` — IDE diagnostics (priority; needs a running IDE backend).
2. If it returns BACKEND_REQUIRED (no IDE), run: `@smells {{ path }}` instead (headless fallback).
@define-end
```

**Verworfene Alternativen (geprüft):** lean-md besitzt bereits `@if`/`@elseif`/`@else`-Container (`eval_condition`). Ein „Probe setzt Var, Makro verzweigt per `@if`"-Muster wäre also verlockend, scheitert aber am Scope: `eval_condition` sieht nur `consumer`/`version`/`shell` (Header), gebundene Makro-Params und `env.NAME` — **nicht** vars.toml-Vars.
- **Weg A** (`@if env.LMD_IDE`): funktioniert mit bestehender Infrastruktur, **bricht aber #498** — der Plan-Task-Text würde maschinenabhängig (Umgebung ≠ committeter Input). Verworfen.
- **Weg B** (`@call inspect(path, "{{ var ide_available }}")` + `@if ide` im Makro): determinismus-konform (Output = f(vars.toml), wie `test_cmd`), bräuchte aber verifiziertes/neues `{{ }}`-Interpolations-Timing für @call-Args (der Var-Wert muss vor der Param-Bindung aufgelöst sein). Über-Engineering für den Nutzen; zurückgestellt als mögliche Folge-Arbeit, falls var-gesteuerte Backend-Präferenz später breiter gebraucht wird.
- **Weg C** (gewählt): der Plan-Text ist immer byte-identisch, die Verzweigung passiert im Agenten zur Task-Zeit — null Infrastruktur, self-healing, #498-sauber.

## 5. `writing-plans`-Body — Authoring mit Code-Intel

Kurzblock in `file-structure` (Blast-Radius bei der Task-Zerlegung) + `write-plan`:

- `@recall <query>` — den Plan aus den in der Brainstorm-Phase gemerkten Spec-Decisions seeden (ctx_knowledge recall), statt den Kontext neu zu erfinden.
- `@graph`/`@impact <symbol>` — vor Task-Grenzen die reale Dependency-Reichweite messen (ctx_callgraph/ctx_impact); begründet, wie invasiv ein Task ist.
- `@find <intent>` — semantisch die Stelle finden, die ein Task ankert (ctx_semantic_search).

Die bestehende Verifikations-Prosa („Verify mit `@read mode=diff`") wird auf die Recipe-Aufrufe umgestellt (`@call verify(...)`, optional `@call review_change()`).

## 5a. Directive-Usage-Referenz für den Plan-Autor (Lücke geschlossen)

**Problem:** Ein Zero-Context-Plan-Autor (das writing-plans-Ethos setzt „knows almost nothing about our toolset" voraus) weiß aus den heutigen Pointern nicht, *was* eine gewobene Directive tut, *wann* er sie nimmt und *wie* die Minimalform lautet. `lang/rust` sagt nur „Symbol nav/refactor: @symbol / ctx_refactor"; die Gloss-Tabelle (`refactor | Refactor: {raw}`) ist ein Render-Audit-Template, **keine** Nutzungs-Referenz. Ohne diese Beschreibung ist die Bindung nominal — der Autor webt Directives, deren Semantik er nicht kennt.

**Lösung:** `tooling/mcp-tools.lmd.md` (bereits der Tool-Disziplin-Pointer des Bodys) wird von der bloßen Mapping-Liste zu einer kompakten **Usage-Referenz** ausgebaut — eine Zeile je gewobener Directive: *Zweck · Minimalform · Wann-nutzen*. Beispiele:

- `@refactor <op> <symbol>` — LSP-sichere Symbol-Umbauten via ctx_refactor. Ops: `rename`/`move`/`extract`. **Nutze** für Symbol-Änderungen in Rust statt `@edit` (`@edit` nur Nicht-Symbol).
- `@review diff-review` — fusioniertes Review-Verdikt (impact + caller + smells) auf einen Diff. **Nutze** als Post-Change-Gate.
- `@smells [scan|summary] <path>` — Code-Smell-Findings (ctx_smells). **Nutze** als Quality-Gate auf geänderte Dateien.
- `@graph <callers|callees|dependents> <symbol>` — Call-/Dep-Graph (ctx_callgraph/ctx_graph). **Nutze** zur Task-Zerlegung & als Refactor-Anker.
- `@impact <symbol>` — Blast-Radius vor Edits (ctx_impact). **Nutze** um Task-Invasivität zu begründen.
- `@recall <query>` / `@remember <content>` — durable Wissen lesen/schreiben (ctx_knowledge). **Nutze** um Spec-Decisions zu seeden bzw. Task-Gotchas zu sichern.

Der Body-Pointer wird präzisiert: `tooling/mcp-tools` ist die **Usage-Referenz**, `gloss/directives` bleibt (korrekt etikettiert) die Render-Gloss-Tabelle. `tooling/mcp-tools` ist Built-in mit Byte-Gate → Seed + Const synchron.

## 6. `plan-template` — Task-Shape-Contract (nicht bloß Demonstration)

**Form-to-Failure (writing-skills-Lens):** Die Baseline-Failure ist ein *Shaping*-Problem — der Plan-Autor produziert Pläne, aber ohne die Bridges. Die wirksame Form dagegen ist ein **positiver Contract, der die Task-Form als Slots-in-Reihenfolge festlegt**, nicht eine gestreute Sammlung „optionaler" `@call`s (weiche Formulierung untergräbt genau das Shaping). Das `plan-template` definiert daher die **verbindliche Verifikations-Sequenz am Task-Ende**:

```
### Verify & Close (jeder Task endet hiermit — feste Reihenfolge)
@call verify("src/foo.rs")                 # 1. Diff inspizieren (immer)
@call reformat_commit("src/foo.rs", "feat: …")  # 2. Format + Commit (immer)
@call remember_decision("foo() ist jetzt die kanonische Increment-Fn")  # 3. durable Fakt (immer)
```

**Bedingte Slots (Conditional auf beobachtbarem Prädikat, nicht „optional"):**
- **bei Symbol-Umbau (rename/move/extract)** → der Task nutzt `@refactor` (§7) und ankert Betroffene via `@call callers("<symbol>")`.
- **bei Änderung an öffentlichem API / > 1 berührter Datei** → `@call review_change()` als Post-Change-Gate.
- **IDE-Backend-Quality-Pass gewünscht** → `@call inspect("src/foo.rs")` (mit `@smells`-Fallback, §4a).

`check_smells`/`recall_context` bleiben im `--signatures`-Index auffindbar für Autoren, die sie brauchen. Der `no_orphan_call`-Gate feuert nur für tatsächlich verwendete `@call`s — ungenutzte Recipes in der Lib sind erlaubt.

## 7. `content/lang/rust.lmd.md` — `@refactor` festhalten

Der Rust-Lang-Pack hält heute dünn „Symbol nav/refactor: @symbol / ctx_refactor". Ausbau zur expliziten **Plan-Inhalt-Regel**: Rust-Tasks mit Rename/Move/Extract weisen `@refactor` (ctx_refactor) an — **keine** Hand-Edits; `@edit` nur für Nicht-Symbol-Änderungen; `reformat` vor Commit via ctx_refactor. So ist die Refactor-Bindung dort verankert, wo der Plan-Autor die Sprach-Konventionen nachschlägt.

`lang/rust` ist Built-in mit byte-genauem Consistency-Gate → **Seed-Datei + `include_str!`-Const synchron ändern**.

## 8. `availability.rs` COVERAGE — Durchsetzung

Neue Zeilen für `lmd-writing-plans` (Format `(skill, step, directive, backing)`):

| step | directive | backing |
|---|---|---|
| `file-structure` | `graph` | `graph_index` (wie brainstorm; `callers`-Op routet outbound zu `ctx_callgraph`) |
| `file-structure` | `impact` | `ctx_impact` |
| `file-structure` | `find` | `ctx_semantic_search` |
| `write-plan` | `recall` | `ctx_knowledge` |
| `plan-format` | `review` | `ctx_review` |
| `plan-format` | `smells` | `ctx_smells` |
| `plan-format` | `reformat` | `ctx_refactor` |

`every_covered_directive_is_registered` prüft, dass jede covered Directive im `default_registry()` existiert → die Bindung ist ab jetzt ein Test, kein Vorsatz. (Directive-Namen wie in `default_registry`; `find` ist zusätzlich schon über `lmd-brainstorm` covered — die eigene writing-plans-Zeile macht die Autoren-Nutzung explizit.)

## 9. 6a-Konformität (Token-Erhalt, zwingend)

Die Makro-Lib-Skalierung aus §6a der Port-Spec bleibt gewahrt — **ohne neuen Gate**, weil die bestehenden Index-Gates generisch über die ganze Datei laufen:

- **`plan_recipes_all_documented`** (generisch): jedes neue `@define` trägt als **erste Body-Zeile** eine `<!-- ... -->`-Beschreibung → erscheint im `--signatures`-Index, Gate bleibt grün. Keine Vorgabe geht „verloren".
- **`no_orphan_call`** (generisch): jedes neue Template-`@call` trifft ein `@define` → Gate bleibt grün.
- **Autoren-Last flach:** der Plan-Autor liest den `--signatures`-Index (~1 Zeile/Makro), nicht die wachsende Lib.
- **Executor-Last null:** `import_library` lädt Defs, schreibt aber keine Bodies — nur `@call`-Expansion landet im Output; der Subagent sieht die Lib nie.

∴ Approach A ist genau *deshalb* token-sicher: mehr Recipes kosten weder Autor noch Executor Tokens.

## 10. Ausgeschlossen (YAGNI)

- `@routes` — web-only Domäne, nicht generisch.
- `@count/@outline` als Recipes — reine Autoren-Orientierung, geringer Wert als eingebettetes Makro.
- `@dispatch/@handoff/ctx_agent/ctx_session` — Koordination, → `lmd-subagent-driven-development` (§2).

(`@inspect` und `@refactor` sind **nicht** mehr ausgeschlossen — siehe §4a bzw. §7.)

## 11. Betroffene Dateien

| Datei | Änderung | Gate |
|---|---|---|
| `content/templates/plan-recipes.lmd.md` + `PLAN_RECIPES`-Const | +8 `@define` (§4/§4a) | `plan_recipes_all_documented`, Fragment-Byte-Gate |
| `content/templates/plan-template.lmd.md` + `PLAN_TEMPLATE`-Const | Task-Shape-Contract: Verify-&-Close-Sequenz + bedingte Slots (§6) | `no_orphan_call`, Fragment-Byte-Gate |
| `content/skills/lmd-writing-plans/body.lmd.md` + Body-Const | Authoring-Block + Verify-Prosa (§5) | Phasen-Isolation, Fragment-Byte-Gate |
| `content/tooling/mcp-tools.lmd.md` + `include_str!`-Const | Directive-Usage-Referenz (§5a) | `builtin_fragments_match_seed_files_on_disk` |
| `content/lang/rust.lmd.md` + `include_str!`-Const | `@refactor`-Regel (§7) | `builtin_fragments_match_seed_files_on_disk` |
| `src/availability.rs` | +7 COVERAGE-Zeilen (§8) | `every_covered_directive_is_registered`, `coverage_rows_writing_plans` |

## 11a. Verhaltens-Pressure-Test (writing-skills Iron Law)

**Warum:** Die statischen Gates (§9) beweisen Doku-*Konsistenz* (jede Recipe dokumentiert, kein verwaistes `@call`, jede Directive registriert) — **nicht**, dass ein Plan-Autor die Bridges tatsächlich nutzt. Die writing-skills-Disziplin verlangt für eine Skill-Verhaltensänderung einen zuerst scheiternden Pressure-Test. Ohne ihn ist die Bindung *behauptet*, nicht bewiesen.

**RED (baseline, vor den Seed-Änderungen):** Ein Plan-Autor-Subagent bekommt eine kleine Beispiel-Spec + den **aktuellen** `lmd-writing-plans` und schreibt einen Plan. Erwartung: der Plan nutzt **keine** der Ziel-Bridges (`verify/review_change/remember_decision/@refactor`) — reproduziert die Ist-Beobachtung aus §1.

**GREEN (nach den Seed-Änderungen):** Derselbe Prompt gegen den **neuen** `lmd-writing-plans`. Erwartung: der generierte Plan enthält die **Verify-&-Close-Sequenz** pro Task (§6) und wendet die bedingten Slots korrekt an (Symbol-Umbau → `@refactor` + `callers`; Multi-Datei → `review_change`).

**REFACTOR:** Falls der Autor die Sequenz umgeht oder Slots auslässt, den Contract im `plan-template`/Body schärfen (Slot strukturell verbindlich statt Prosa), bis GREEN hält — nicht die Erwartung senken.

Dieser Test ist ein **Plan-Task**, kein `cargo`-Gate (er bewertet LLM-Output, nicht Code); das Ergebnis (RED-Transkript + GREEN-Transkript) wird als durable Fakt via `@remember` gesichert.

## 12. Erfolgskriterien

- **Verhaltens-Nachweis (primär):** RED→GREEN aus §11a belegt — Baseline ohne Bridges, generierter Plan mit Verify-&-Close-Sequenz + korrekten bedingten Slots.
- `cargo nextest run` grün (inkl. der generischen Index-Gates + neuem COVERAGE-Gate).
- `lean-md render plan-recipes.lmd.md --signatures` listet alle neuen Recipes mit Doc-Zeile.
- Ein frisch generierter Beispiel-Plan rendert pro Task fehlerfrei (Verify-&-Close-Sequenz + bedingte Slots).
- `cargo clippy -D warnings` sauber; #498-Determinismus (byte-stabile Seeds) gewahrt.
