# lmd-brainstorm — Bridge-Bindung an lean-md/lean-ctx angleichen (Design)

**Datum:** 2026-07-03
**Status:** Entwurf zur Review
**Kontext:** Schwester-Spec zu `2026-07-03-lmd-plan-lean-ctx-binding-design.md` (writing-plans). Beobachtung dort: generierte Pläne nutzten nur einen Bruchteil der lean-md-Bridges. Frage für **diesen** Spec: Sollte dieselbe stärkere Bindung (`@find @smells @graph …`) auch in `lmd-brainstorm` erfolgen? Antwort: **teilweise** — die Scope-Grenze ist Design-Zeit vs. Task-Zeit, und die realen Lücken sind klein.

---

## 1. Problem & Ist-Zustand

`lmd-brainstorm` bindet Bridges bereits — aber inkonsistent zwischen COVERAGE (Registrierung) und Body-Prosa (Demonstration).

**COVERAGE (`availability.rs`), registriert:**

| step | directive | backing |
|---|---|---|
| `explore` | read / list / search / **find** | ctx_read / ctx_tree / ctx_search / ctx_semantic_search |
| `approaches` | **graph** / **impact** | graph_index / ctx_impact |
| `write-spec` | edit / **remember** | ctx_edit / ctx_knowledge |
| `self-review` | **review** / dispatch | ctx_review / fragment-compose |
| `handoff` | dispatch / handoff | fragment-compose / ctx_handoff |

**Body-Prosa, tatsächlich gewoben:**

- `explore`: `@list`/`@search`/`@read` + `@graph`/`@impact` — **`@find` fehlt**, obwohl covered.
- `approaches`: `@graph`/`@impact`.
- Documentation (`write-spec`): `@edit`/`@remember`.
- `self-review`: `@dispatch` → spec-reviewer — **`@review` fehlt**, obwohl covered.

**Kernbefund:** `graph`/`impact` sind real gewoben. `find` und `review` sind **nominal covered, aber nicht demonstriert** — genau die „nominal binding"-Lücke, vor der die writing-plans-Spec §5a warnt. `smells`/`reformat`/`inspect`/`verify` fehlen ganz — teils zu Recht (siehe §2).

## 2. Scope-Grenze (gesetzt): Design-Zeit vs. Task-Zeit

Die entscheidende Einsicht: nicht jede writing-plans-Bridge gehört nach brainstorm. brainstorm produziert einen **Spec**, keinen Diff. Bridges, die auf einem **Change/Diff** operieren, haben zur Design-Zeit kein Ziel.

| writing-plans-Bridge | Rolle (Task-Zeit) | brainstorm (Design-Zeit) |
|---|---|---|
| `@graph` / `@impact` | Task-Blast-Radius | **schon gewoben** (explore + approaches) ✅ |
| `@find` | Task semantisch ankern | COVERAGE hat's, **Prosa fehlt** → §3 |
| `@remember` | Task-Gotcha sichern | schon in write-spec ✅ (brainstorm ist der *Writer*) |
| `@recall` | Plan aus Spec-Decisions seeden | *Reader-Seite* → gehört zu writing-plans (symmetrisch, nicht duplizieren) |
| `@smells` `@review` `@inspect` `@reformat` `@verify` | Change-Gates auf einem Diff | **N/A** — kein Diff zur Design-Zeit |

∴ Der korrekte Design-Zeit-Satz ist bereits `@find @graph @impact` (+ `@remember` Write-Seite). Die Change-Gates sind **bewusste Auslassungen** (Transparenz via GAP_LIST, §5), keine Löcher.

**Symmetrie `@remember`/`@recall`:** brainstorm merkt Spec-Decisions (`@remember`, write-spec); writing-plans zieht sie (`@recall`, Plan-Seeding). Diese Rollen-Trennung ist beabsichtigt und bleibt.

## 3. Lücken schließen

### 3.1 `@find` in explore-Prosa weben

COVERAGE hat `explore/find→ctx_semantic_search`, aber die Prosa nennt nur `@list/@search/@read`. Der explore-Bullet wird:

> Explore with `@list`/`@search`/`@read` (structural) **and `@find` (semantic locate — ctx_semantic_search)** before asking questions; gauge a change's blast radius with `@graph`/`@impact`.

### 3.2 `self-review/review→ctx_review`-Row entfernen

`ctx_review` reviewt **Code-Diffs** (impact + caller + smells). Der brainstorm-`self-review` reviewt **Spec-Prosa**, real über `@dispatch → spec-reviewer` (eigene COVERAGE-Row). Die `review`-Row ist damit eine **Fehl-Registrierung**. **Entscheidung: entfernen.** `self-review` bleibt über die `dispatch`-Row covered; kein Verhalten geht verloren.

## 4. Usage-Referenz (DRY über den geteilten Seed)

writing-plans-Spec §5a baut `content/tooling/mcp-tools.lmd.md` ohnehin von der Mapping-Liste zur **Usage-Referenz** aus (je Directive: Zweck · Minimalform · Wann). brainstorms Hard-Rules-Pointer (`@include hard-rules`) zeigt **bereits** auf `tooling/mcp-tools` — **dieselbe Datei bedient beide Skills**. Kein neuer File, kein Duplikat.

Nötig ist nur, dass die §5a-Referenz die **Design-Zeit-Framing-Zeile** für `@find` trägt (heute fehlt `@find` in der §5a-Beispielliste, die `@graph`/`@impact`/`@remember` schon abdeckt):

> - `@find <intent>` — semantische Lokalisierung via ctx_semantic_search. **Nutze** zur Design-Zeit, um die Stelle zu finden, die ein Design/Task ankert; strukturell (Keyword/Pfad) → `@search`.

Diese eine Zeile schließt zugleich die REFACTOR-Rationalisierung aus §6 („`@search` reicht doch"): sie trennt strukturell (`@search`) von semantisch (`@find`) explizit.

**Koordination mit writing-plans-Spec:** Die `@find`-Zeile in `tooling/mcp-tools` ist ein **gemeinsamer** Bearbeitungspunkt. Falls beide Ports zeitlich versetzt laufen, trägt der zuerst laufende die Zeile ein; der zweite verifiziert nur Präsenz + Byte-Gate. Kein Doppel-Eintrag.

## 5. Durchsetzung — COVERAGE + GAP_LIST

- **COVERAGE:** `explore/find`-Row **behalten** (jetzt auch in Prosa demonstriert). `self-review/review`-Row **entfernen** (§3.2). **Keine** neuen Change-Gate-Rows.
- **GAP_LIST** (heute `["ctx_benchmark", "ctx_package", "ctx_provider"]`) += transparente Design-Zeit-Auslassungen dokumentieren: `ctx_smells`, `ctx_review` (Code-Diff), `ctx_refactor reformat` sind **Task-Zeit-Gates**, bewusst nicht im brainstorm-Design-Zeit-Pfad. Der Datei-Header (`availability.rs` Z. 4) fordert genau das: „transparency, not a silent hole".
- Gates bleiben grün: `every_covered_directive_is_registered` (removed row = weniger zu prüfen), `gap_list_rendered` (deckt die neuen GAP-Einträge), `coverage_carries_*` (unberührt — betreffen dispatch/companion-Rows).

## 6. Verifikation — Split nach Änderungsart

Die Änderungen zerfallen in zwei Klassen mit unterschiedlicher Verifikation.

### A) Skill-Body-/Prosa-Edits → `lmd-writing-skills` (RED→GREEN→REFACTOR)

Betrifft: `@find`-Weave in explore (§3.1) + Usage-Ref-Zeile (§4). Die Iron Law („NO SKILL WITHOUT A FAILING TEST FIRST") gilt **explizit für Edits**.

1. **RED — Baseline-Pressure-Test zuerst.** Szenario: ein Brainstorming-Agent exploriert eine Codebase, in der die ankernde Stelle nur **semantisch** (nicht per Keyword/Pfad) findbar ist. Ohne den Edit greift er zu `@search` oder gibt auf → **beobachteter Fehlschlag**. Ohne diesen beobachteten Fail ist nicht bewiesen, dass der `@find`-Weave das Richtige lehrt.
2. **GREEN — minimaler Edit:** `@find`-Zeile in explore-Prosa + Usage-Ref-Zeile → Agent greift zu `@find`.
3. **REFACTOR — Rationalisierung schließen:** die strukturell-vs-semantisch-Trennung in der Usage-Ref (§4) entkräftet „`@search` reicht doch".

### B) Rust-/Gate-Edits → `cargo nextest run` + Byte-Gates

Betrifft: `review`-Row entfernen (§3.2), GAP_LIST-Zeilen (§5).

- `cargo nextest run` (nie `cargo test`): `every_covered_directive_is_registered`, `gap_list_rendered`, `coverage_carries_*` grün.
- **Byte-Konsistenz-Gate** für jeden angefassten embedded Seed (`tooling/mcp-tools`; brainstorm-body falls `include_str!`-embedded): Seed-Datei **und** Const synchron ändern, sonst rot (#498).
- **Render-Smoke:** `cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase explore --consumer=ai` zeigt `@find`; Ausgabe byte-stabil.

### Ausführungs-Reihenfolge

**A vor B.** Erst der beobachtete Fail (RED) rechtfertigt den Body-Edit (GREEN); danach zieht die Durchsetzung (COVERAGE/GAP_LIST + Gates) nach. Der `pre-commit`-`cargo fmt` (pro geänderter Datei) bleibt Pflicht.

## 7. Nicht-Ziele (YAGNI)

- **Kein** `@smells`/`@review`/`@reformat`/`@inspect`/`@verify` in brainstorm — Change-Gates ohne Design-Zeit-Ziel (§2). Als GAP dokumentiert, nicht gewoben.
- **Kein** `@recall` in brainstorm — Reader-Seite lebt in writing-plans; brainstorm ist die `@remember`-Write-Seite.
- **Kein** neuer Usage-Referenz-File — der geteilte `tooling/mcp-tools`-Seed (§4) trägt beide Skills.
- **Kein** neuer Gate — die bestehenden generischen Gates (§5) decken die Änderung.

## 8. Reales Delta (Zusammenfassung)

1. `content/skills/lmd-brainstorm/body.lmd.md` — eine `@find`-Zeile in der explore-Prosa (+ Seed/Const-Sync falls embedded).
2. `content/tooling/mcp-tools.lmd.md` — eine `@find`-Usage-Zeile (Seed + Const-Sync, Byte-Gate).
3. `src/availability.rs` — `self-review/review`-Row **raus**, GAP_LIST += `ctx_smells`/`ctx_review`/`reformat`.
4. Pressure-Test (RED-Artefakt) für den `@find`-Weave gemäß `lmd-writing-skills`.

Vier kleine, begründete Eingriffe — die Antwort auf „auch smells/graph binden?" ist **nein** (graph schon da, smells design-fremd); das echte Fix ist die `@find`-Konsistenz plus Transparenz für die bewussten Auslassungen.
