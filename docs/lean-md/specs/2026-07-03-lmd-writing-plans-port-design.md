# lmd-writing-plans — Voll-Port von superpowers:writing-plans (Design-Spec)

**Datum:** 2026-07-03 · **Branch:** feat-lmd-v2 · **Status:** Design (brainstorm-Ausgang)

**Goal:** `superpowers:writing-plans` verlustfrei als nativen lean-md-Skill
`lmd-writing-plans` ausliefern — phasen-isoliert wie `lmd-brainstorm` — und dabei
das erzeugte **Plan-Format** auf lean-md umstellen, sodass Pläne token-effizient
werden **ohne Funktionsverlust**. Der Skill produziert `.lmd.md`-Pläne mit
task-on-demand-Rendering, Code-Ankern für existierenden Code und
`@define`/`@call`-Makros für Boilerplate. Nutzer-Anpassung läuft über das
bestehende `PROJECT_SEEDS`/`vars.toml`-Template-System.

**Quelle:** `~/.claude/plugins/cache/claude-plugins-official/superpowers/6.1.1/skills/writing-plans/`
(`SKILL.md` + `plan-document-reviewer-prompt.md`; **keine** `scripts/`-Assets).

**Vorlage:** Schwester-Skill `lmd-brainstorm`
(`docs/lean-md/specs/2026-06-30-lmd-brainstorm-port-design.md` +
`docs/lean-md/plans/2026-06-30-lmd-brainstorm-port*.md`) — Port-Muster, Seed-/
Companion-/Test-Struktur, Reference-Closure.

---

## 1. Zwei Ebenen — bewusst getrennt

Dieses Vorhaben hat zwei Ebenen, die nicht vermischt werden dürfen:

- **Ebene A — Skill-Port:** `writing-plans` → `lmd-writing-plans` als
  phasen-isolierter, binär-eingebetteter Skill (Muster: `lmd-brainstorm`).
  Verlustfrei: jede Original-`SKILL.md`-Sektion landet in genau einer lmd-Phase.
- **Ebene B — Plan-Format:** *Was* der Skill als Plan erzeugt. superpowers schreibt
  bewusst redundant (repeat-the-code, complete code in every step, no placeholders)
  für kontextlose Subagenten. lean-md-Subagenten haben lean-ctx (warmer Cache,
  `@symbol`/`ctx_read`) → Code JIT ladbar statt im Plan getragen. Ebene B ersetzt
  die Verbatim-Doktrin durch ein `.lmd.md`-Plan-Format (siehe §5).

„Ohne Funktionsverlust" gilt für BEIDE Ebenen: die Skill-Anleitung bleibt
vollständig, und der ausführende Subagent erhält beim Rendern denselben
Informationsgehalt wie bei einem Verbatim-Plan (die Makros expandieren zu Klartext,
die Anker werden JIT aufgelöst).

## 2. Skill-Oberfläche (analog brainstorm)

| Artefakt | Zweck |
|---|---|
| `content/skills/lmd-writing-plans/SKILL.md` | Delegations-Stub: Frontmatter + Phasenliste + Companion-/Seed-Pointer |
| `content/skills/lmd-writing-plans/body.lmd.md` | phasen-isolierte `@phase`-Blöcke (7 Phasen, §3) |
| `content/skills/lmd-writing-plans/companions/plan-reviewer.lmd.md` | Port von `plan-document-reviewer-prompt.md`; Review-Dispatch-Brief |
| `content/templates/plan-recipes.lmd.md` | **neuer PROJECT_SEED**: anpassbare `@define`-Makro-Lib (§6) |
| `content/templates/plan-template.lmd.md` | **neuer PROJECT_SEED**: `.lmd.md`-Plan-Skelett zum Kopieren |

**Platzierungs-Entscheidung:** Repo-**Quelle** unter `content/templates/` (wie
`dispatch-contract.ext.lmd.md`); **PROJECT_SEEDS-Zielpfad flach im Root** von
`.lean-ctx/lean-md/` — also `.lean-ctx/lean-md/plan-recipes.lmd.md` +
`.lean-ctx/lean-md/plan-template.lmd.md` (kein Unterordner). Das ist exakt das Muster
von `dispatch-contract.ext`, dessen Seed ebenfalls aus `content/templates/` in den
contracts_dir-**Root** materialisiert. `plan-recipes` kann intern `@import lang/<lang> /`
für sprachspezifische Default-Makros ziehen.

**Kein** neues Discipline-Gate-Fragment: `writing-plans` ist kein HARD-GATE-Skill.

## 3. Phasen — Fidelity-Mapping der superpowers-Sektionen

Verlustfreie Abbildung jeder `SKILL.md`-Sektion auf genau eine Phase:

| Phase | portiert aus superpowers-Sektion |
|---|---|
| `pre-context` | Overview + „Announce at start" + Scope Check. **Statt** `@include hard-rules`: Verweis auf das Direktiv-Vokabular (`gloss/directives`, `tooling/mcp-tools`) + das lang-pack **der Projektsprache** (§3a). |
| `file-structure` | File Structure (Datei-Landkarte vor Task-Zerlegung) |
| `task-sizing` | Task Right-Sizing + Bite-Sized Task Granularity |
| `plan-format` | **umformuliert:** Plan Document Header + Task Structure + No Placeholders + Remember — auf das `.lmd.md`-Format (§5). Verweist auf `plan-template` + `plan-recipes`. |
| `write-plan` | Schreiben nach `docs/lean-md/plans/YYYY-MM-DD-<feature>.md` (user prefs override) |
| `self-review` | Self-Review-Checkliste (Spec-Coverage / Placeholder-Scan / Type-Consistency) + `@dispatch … companion="plan-reviewer" role=review` |
| `handoff` | Execution Handoff → Name-Pointer `lmd-subagent-driven-development` / `lmd-executing-plans` (§8) |

Jede Phase endet mit `next: render phase "<n>"`; `handoff` ist terminal.

### 3a. Sprachwahl im `pre-context` — projektabhängig, nicht wahllos

Der `pre-context` verweist **nicht** auf einen abstrakten `lang/<lang>`-Platzhalter,
sondern auf das lang-pack der **tatsächlichen Projektsprache**: in einem Rust-Projekt
`lang/rust`, in einem Python-Projekt `lang/python` usw. Der Skill-Body erkennt die
Projektsprache (Manifest/Dateiendungen — via `@list`/`@search` beim Explorieren) und
referenziert das passende Pack.

**Verfügbarkeit (ehrlich):** Aktuell existiert **nur `content/lang/rust.lmd.md`**.
Weitere lang-packs (`python`, …) sind **Folge-Arbeit** und werden erst angelegt, wenn
ein Projekt sie braucht. Fehlt das Pack einer Sprache, arbeitet der Autor ohne
lang-spezifische Konventionen weiter (kein Hard-Fail) und `plan-recipes` fällt auf
generische Makros zurück. Der materialisierte `plan-recipes.lmd.md` trägt das
projektpassende `@import lang/<sprache> /` — für dieses Repo `lang/rust`; ein
Python-Projekt setzt es auf `lang/python`, sobald das Pack existiert.

## 4. Neue Code-Fläche: file+phase-Render **mit vars-Prepass**

Voraussetzung für Ebene B. Heute rendert der Datei-Pfad
(`cmd_render` → `do_render` → `engine::render_with_overrides`) ein Dokument als
Ganzes und macht **keinen** `@var`-Prepass. Die Phasen-Isolationsmaschinerie
(`phases::capture_phase_bodies` + `EngineContext::phase_body`) und der vars-Prepass
(`skill_vars::load_vars` + `scan_var_decls`) existieren bereits — nur der Datei-Pfad
verdrahtet sie nicht.

**Änderung:** `render <file.lmd.md> --phase P` unterstützen. Der Datei-Pfad muss —
exakt wie `render_skill` — (a) die Ziel-Phase isolieren *und* (b) den vars-Prepass +
`load_vars(jail_root)` ausführen, damit `@var`/`vars.toml`/`@import` in erzeugten
Plänen greifen.

- **Kein** Render-Core-Change (`rushdown`/`evalexpr`, Bridges bleiben unberührt).
- Betroffen: `src/bin/lean_md.rs` (`cmd_render`, `parse_render_flags` bereits vorhanden)
  + eine pub-Render-Funktion, die arbiträren Source + Phase + vars-Prepass rendert
  (analog `render_skill`, aber quell-agnostisch).
- **Jail-Semantik (Entscheidung):** Heute setzt `load_file` `jail_root` = Parent der
  Datei. Ein Plan liegt in `docs/lean-md/plans/`, die Nutzer-Seeds in
  `.lean-ctx/lean-md/` (Projekt-Root) — mit Datei-Parent-Jail wären `vars.toml` +
  `@import`-Ziele **nicht** auflösbar. Daher setzt der file+phase-Render `jail_root`
  = **cwd (Projekt-Root)**, nicht Datei-Parent — analog `render_skill`
  (`std::env::current_dir`). Der Plan-Pfad wird relativ zum cwd aufgelöst. Diese
  Abweichung vom `load_file`-Default ist bewusst und im Plan zu testen (Seed-
  Auflösung aus einem Unterordner-Plan heraus).

## 5. Das erzeugte Plan-Format (Ebene B, der Token-Hebel)

Ein Plan ist ein `.lmd.md`:

- **Meta-Kopf** (Body-Top, außerhalb der Phasen): Goal / Architecture / Global
  Constraints — einmal, via `@include` referenzierbar; `@var`-Deklarationen +
  `@import .lean-ctx/lean-md/plan-recipes /` für das Makro-Vokabular.
- **Eine `@phase "task-N"` pro Task.** Der ausführende Controller rendert nur die
  aktuelle Task: `lean-md render <plan.lmd.md> --phase task-N` → Phasen-Isolation
  liefert genau diesen Task-Block (kein Cross-Task-Leak).
- **No-loss-Regel innerhalb einer Task (verbindlich):**
  - **existierender** Code → Anker (`@symbol name` / `@read path mode=signatures` /
    `path:line`), JIT aus warmem Cache aufgelöst — **kein** Verbatim-Duplikat.
  - **neuer** Code (existiert noch nicht) → **verbatim** (wie superpowers).
  - Interfaces / Consumes-Produces / Commands / „Expected:" → **verbatim streng**
    (No-Placeholders bleibt für Intent/Schnittstellen).
  - Verifikation → `@read mode=diff` statt Copy-Paste-Kontrolle.
- **Boilerplate** (TDD-Zyklus, commit, test-run) → `@call <recipe>(...)`; expandiert
  beim Rendern zu Klartext → **kein Funktionsverlust** für den Ausführer.

**Warum das no-loss ist:** `render --phase task-N` expandiert `@call` zu vollem
Text und der Subagent löst Anker via lean-ctx auf; er sieht denselben Inhalt wie
bei einem Verbatim-Plan. Der Token-Gewinn liegt im **Quelltext** (Autoring /
Speicher / Review) und in **DRY** (Boilerplate einmal definiert) + **on-demand**
(nur Task N statt ganzer Plan im Kontext).

## 6. Nutzer-Anpassung — vier Ebenen über `.lean-ctx/lean-md/`

Alle vier Mechanismen existieren bereits im Code (verifiziert):

1. **`@var` + `vars.toml`** — Werte-Override; Präzedenz **vars.toml > Inline-Default**
   (`var.rs::declaration_does_not_override_config`; `skill_vars::load_vars`).
   Vorbereitet: `test_cmd = "cargo nextest run"`.
2. **`@import .lean-ctx/lean-md/plan-recipes /`** — lädt die lokale Makro-Lib
   unsichtbar in die Registry (`macros::extract_definitions`, „last `@define` wins";
   jailed relativer Pfad). Hier definiert/überschreibt der Nutzer eigene Makros.
3. **`@define`/`@call`** — `{{ param }}`-Substitution; passiv (Text) + aktiv
   (dispatcht `@symbol`/`@read`); kann `{{ test_cmd }}` referenzieren.
4. **`overlay_body`** — `.lean-ctx/lean-md/skills/<name>/body.lmd.md` überschreibt
   den ganzen Skill-Body (grober Hebel, selten nötig).

**`test`-Makro-Beispiel (verbindlich):** `plan-recipes.lmd.md` enthält ein per
`@call test(...)` aufrufbares Makro (Hinweis: `@task` ist **kein** lean-md-Direktiv —
der Mechanismus ist `@define`/`@call`):
```
@define test(name)
Run: `{{ test_cmd }} {{ name }}`
@define-end
```
→ default `cargo test`, Nutzer-Override `cargo nextest run` via `vars.toml`, **ohne**
Skill- oder Plan-Edit.

**PROJECT_SEEDS-Integration:** `plan-recipes.lmd.md` und `plan-template.lmd.md`
(Quelle `content/templates/`, Zielpfad Root, §2) werden neue Einträge in
`seeds.rs::PROJECT_SEEDS`. `materialize_contracts` kopiert sie **absent-only** in den
Root von `.lean-ctx/lean-md/`
(Nutzer-Edits nie überschrieben); der `FragmentRegistry`-Resolver bevorzugt die
lokale Datei über den eingebetteten Seed. `plan-recipes` kann per
`@import lang/rust /` sprachspezifische Default-Makros erben.

**Install-Verdrahtung (neuer Arbeitsschritt):** `materialize_contracts` ist heute
**ungenutzt** (nur in `seeds.rs` definiert + getestet, kein Aufrufer). `install_skill`
materialisiert bislang nur skill-scoped `ASSETS` nach `.claude/skills/<name>/`, **nicht**
die PROJECT_SEEDS. Damit `lean-md skill install lmd-writing-plans` das lokale Template
etabliert, muss `install_skill` (bzw. der CLI-`skill install`-Pfad) zusätzlich
`materialize_contracts(project_root, ".lean-ctx/lean-md")` aufrufen → schreibt
`plan-recipes.lmd.md` + `plan-template.lmd.md` (Root von contracts_dir) **absent-only**.
Dies ist eine bewusste Verhaltenserweiterung von `install_skill` und im Plan zu testen
(Install schreibt beide Seeds; zweiter Lauf idempotent; Nutzer-Edits unberührt).

**plan-template = selbst-dokumentierend (verbindlicher Inhalt):** Der materialisierte
`plan-template.lmd.md` enthält **auskommentierte Beispiele**, die dem Nutzer zeigen, wie
ein `.lmd.md`-Plan aussieht/aussehen **muss** — der Meta-Kopf (Goal/Architecture/Global
Constraints + `@var`-Decls + `@import`), ein `@phase "task-N"`-Musterblock mit
Anker-Beispiel (existierender Code) vs. Verbatim (neuer Code) und `@call test(...)` /
`@call commit(...)`-Aufrufen. Kommentare erklären jede Zeile; der Nutzer kopiert den
Block und entkommentiert/passt an. `plan-recipes.lmd.md` ist analog kommentiert
(jedes `@define` mit einer Beschreibungszeile). Ziel: das Format ist aus der
materialisierten Datei heraus verständlich, ohne den Skill-Body zu lesen.

### 6a. Makro-Lib-Skalierung — Signatur-Index + Gate (Token-Erhalt)

**Problem:** Die Render-Ersparnis ist strukturell garantiert (verifiziert:
`import_library` lädt alle `@define`s in die `MacroRegistry`, schreibt aber **keine**
Bodies in den Output — nur per `@call` aufgerufene Makros expandieren; der ausführende
Subagent sieht die Lib nie). Die **Autoring**-Last skaliert dagegen mit der Lib-Größe:
um zu wissen, *welche* Makros existieren, müsste der Autor sonst die ganze
(wachsende / künftig geteilte) `plan-recipes` lesen → die Ersparnis zerfällt.

**Signatur-Modus (neue CLI/Engine-Fläche):** `lean-md render <lib.lmd.md> --signatures`
(Flag-Name im Plan finalisieren) extrahiert je `@define` nur den Kopf
`name(params)` + die unmittelbar folgende **Beschreibungszeile** (Konvention), **ohne**
Body — ein kompakter „Makro-API"-Index (~1 Zeile/Makro). Der Plan-Autor liest diesen
Index statt der Lib. Basis existiert (`parse_call_signature`, `extract_definitions`,
`MacroRegistry`); die Signatur-Projektion (Header + Doc-Zeile, Bodies gestrippt) ist neu.

**Index-Gate (Consistency, Token-Erhalt absichern):**
- **Vollständigkeit:** jede `@define` in `plan-recipes` trägt eine Beschreibungszeile —
  sonst fehlt sie im Index (das Gate bricht). So geht keine Vorgabe „verloren".
- **Kein verwaistes `@call`:** jedes `@call` im `plan-template` (und in Beispiel-Plänen)
  trifft ein existierendes `@define`. Ergänzt das bestehende Laufzeit-Verhalten
  (`@call unknown` → sichtbarer `macro not found`) um einen **statischen** Check.

**Repo-übergreifend = Folge-Arbeit (§12):** `@import` ist auf `jail_root` gejailt; eine
geteilte/globale Lib bräuchte einen globalen Suchpfad jenseits des Jails — Architektur-
Erweiterung, nicht Teil dieses Ports. Der Signatur-Index gilt dann unverändert.

**Subplan-Zuordnung:** Signatur-Modus → Subplan 1 (CLI/Engine-Fundament, neben
file+phase-Render); Index-Gate → Subplan 4 (Templates).

## 7. hard-rules abspecken (übergreifender Arbeitsstrang)

Die generische `hard-rules`-Prosa überlappt fast vollständig mit den konkreten
Seeds — sie bleibt nur für *freies* Agenten-Handeln (ohne Direktive) relevant:

| hard-rules-Zeile | abgedeckt durch |
|---|---|
| „I/O only via lean-ctx (@read/@search/@list/@query)" | `tooling/mcp-tools.lmd.md` |
| „Never native cat/grep; never `raw`" | **einzige nicht-redundante Zeile — behalten** |
| „*.rs symbol-aware: @symbol/ctx_refactor" | `lang/rust.lmd.md` |
| „@edit non-symbol; reformat before commit" | `lang/rust.lmd.md` |

**Änderung:** `content/core/hard-rules.lmd.md` auf die eine nicht-redundante Zeile
reduzieren + Ein-Zeilen-Pointer auf `tooling/mcp-tools` + `lang/<lang>`.

**Blast-Radius (verifiziert):**
- `hard-rules` ist Built-in mit **byte-genauem Consistency-Gate**
  (`fragments::builtin_fragments_match_seed_files_on_disk`) → Seed + Const synchron
  ändern.
- `@dispatch` bindet `hard-rules` in den Dispatch-Contract ein
  (`dispatch.rs:113`) → der Ausführer-Contract-Output ändert sich; Dispatch-Tests
  prüfen, dass die Kern-Disziplin erhalten bleibt.
- Konsumenten: `lmd-brainstorm` (pre-context/Discipline-Phasen), `lmd-writing-skills`,
  `lmd-test-driven-development` includen `hard-rules` — gerenderte Ausgaben ändern
  sich; inhaltsprüfende Tests (`builtin_resolves_before_file` verlangt
  `contains("lean-ctx")`) müssen grün bleiben.
- **Dieser Strang betrifft ALLE Skills** — im Implementierungsplan als eigene,
  zuerst abgeschlossene Task mit voller Regressions-Suite führen.

## 8. Reference-Closure (verbindlich für Seeds + Companions)

| Original-Verweis | lmd-Ziel |
|---|---|
| `writing-plans` / „the writing-plans skill" | `lmd-writing-plans` |
| `superpowers:subagent-driven-development` | `lmd-subagent-driven-development` (Name-Pointer, Folge-Spec) |
| `superpowers:executing-plans` | `lmd-executing-plans` (Name-Pointer, Folge-Spec) |
| `superpowers:using-git-worktrees` | **entfällt** (Projekt-Policy „no worktrees") |
| `docs/superpowers/plans/` | `docs/lean-md/plans/` (user prefs override) |
| `plan-document-reviewer-prompt.md` | Companion `plan-reviewer` |
| „frontend-design, mcp-builder …" | „any implementation skill" (kein superpowers-Name) |

**Grep-Gate:** kein „superpowers"-String in Seeds/Companions. Die Name-Pointer
`lmd-subagent-driven-development` / `lmd-executing-plans` sind **Platzhalter** —
diese Skills werden in Nachfolge-Specs portiert (exakt wie `lmd-writing-plans` ein
Name-Pointer im brainstorm-`handoff` war, bevor es diesen Skill gab).

## 9. Registrierungs-Flächen

| Datei | Änderung |
|---|---|
| `src/skills.rs` | `LMD_WRITING_PLANS_BODY` const (`include_str!`) + `SKILLS`-Zeile + `COMPANIONS`-Zeile (`plan-reviewer`) |
| `src/seeds.rs` | `PROJECT_SEEDS` += Zielpfade `plan-recipes.lmd.md` + `plan-template.lmd.md` (Root), Quelle `include_str!("../content/templates/…")` |
| `src/skill_install.rs` | `INSTALLABLE_SKILLS` += `lmd-writing-plans` + `WRITING_PLANS_SKILL_MD` const |
| `src/availability.rs` | `COVERAGE`-Zeilen (Phasen→Direktiven→Backing) + ggf. Companion/Dispatch-Zeile |
| `src/bin/lean_md.rs` | file+phase-Render + vars-Prepass (§4); Makro-Signatur-Modus (§6a) |
| `src/macros.rs` | Signatur-Projektion (`@define`-Header + Doc-Zeile, Bodies gestrippt) auf `MacroRegistry` (§6a) |
| `content/core/hard-rules.lmd.md` + `src/fragments.rs` | hard-rules abspecken + Const-Sync (§7) |

## 10. Test-Gates

| Gate | prüft |
|---|---|
| `skill_registered` / `all_phases_render_nonempty` | 7 Phasen rendern; `skill_body` Some |
| `phase_isolation` | kein Cross-Phase-Leak über 7 Phasen |
| `companion_render` | `plan-reviewer` non-empty; `phase`⊕`companion` |
| `dispatch_plan_reviewer_composes` | Contract + Reviewer-Brief + `role=review` |
| `reference_closure_grep` | kein „superpowers" in Seeds/Companions |
| `coverage_rows` | availability.rs-Zeilen registriert |
| **`file_phase_render`** (neu) | `render <file> --phase` isoliert korrekt |
| **`file_phase_vars_prepass`** (neu) | `@var`/`vars.toml` greifen im Datei-Pfad (`test_cmd`-Override sichtbar) |
| **`plan_recipes_import`** (neu) | `@import plan-recipes` + `@call test(...)` expandiert mit vars-Wert |
| **`macro_signatures_extract`** (neu) | `render <lib> --signatures` liefert `@define`-Header + Doc-Zeile, **keine** Bodies |
| **`plan_recipes_all_documented`** (neu) | jede `@define` in `plan-recipes` trägt eine Beschreibungszeile (Index-Vollständigkeit) |
| **`no_orphan_call`** (neu) | jedes `@call` in `plan-template` trifft ein existierendes `@define` (statisch) |
| `project_seeds_materialize` | `plan-recipes` + `plan-template` absent-only materialisiert, idempotent |
| **`install_wires_seeds`** (neu) | `skill install lmd-writing-plans` schreibt beide Seeds in den Root von `.lean-ctx/lean-md/`; zweiter Lauf idempotent; Nutzer-Edits unberührt |
| **`plan_template_self_documents`** (neu) | `plan-template.lmd.md` enthält auskommentierte Muster-Beispiele (Meta-Kopf, `@phase "task-N"`, `@call`); ein entkommentiertes Beispiel rendert fehlerfrei |
| **`hard_rules_slim`** (neu) | abgespeckter Seed == Const (Consistency); Kern-Disziplin erhalten |
| `dispatch_contract_regression` | Dispatch-Contract nach hard-rules-Abspeckung weiterhin korrekt |
| Determinismus (#498) | built-in == on-disk Seeds; `CliBackend` == `McpBackend` |
| Full-Gate | `cargo fmt`, `cargo nextest run`, clippy `-D warnings`, Render-Smoke |

## 11. Scope / erwartete Plan-Dekomposition

Zu groß für einen einzelnen Plan → der Implementierungsplan zerlegt (analog
`lmd-brainstorm`s 4 Subpläne) in eigenständig grün-testbare Einheiten:

- **Subplan 1 — Fundament (Voraussetzung):** file+phase-Render + vars-Prepass (§4)
  + Jail-Semantik-Entscheidung. Liefert die Render-Fähigkeit, auf der Ebene B ruht.
- **Subplan 2 — hard-rules abspecken (§7):** übergreifend, ALLE Skills betroffen →
  isoliert, zuerst, mit voller Regressions-Suite (Consistency-Gate + Dispatch-Contract
  + brainstorm/writing-skills/tdd-Render grün).
- **Subplan 3 — Skill (Ebene A):** `body.lmd.md` (7 Phasen) + `SKILL.md`-Stub +
  `plan-reviewer`-Companion + Registrierung (`skills.rs`/`skill_install.rs`).
- **Subplan 4 — Templates + Abschluss:** `plan-recipes` + `plan-template` als
  PROJECT_SEEDS + Materialisierung + `COVERAGE` + Reference-Closure-Grep +
  Determinismus + Full-Gate.

Reihenfolge: 1 und 2 sind unabhängig (parallelisierbar); 3 setzt 1 voraus; 4 setzt
1–3 voraus. Die endgültige Zahl/Schnittführung entscheidet `lmd-writing-plans`.

## 12. Non-Goals

- Kein Port von `lmd-subagent-driven-development` / `lmd-executing-plans` (Folge-Specs).
- Kein Render-Core-Change (`rushdown`/`evalexpr`, Bridges).
- Keine projektspezifischen Makros im Skill hardcodiert (die liefert `plan-recipes`
  als anpassbaren Seed; sprachspezifische Defaults via `lang/<lang>`).
- Kein Worktree-Support (Projekt-Policy).
- Keine Skill-Discovery-Targets für fremde Harnesses.
- **Keine repo-übergreifende / globale Makro-Lib** (globaler `@import`-Suchpfad jenseits
  `jail_root`) — Folge-Arbeit (§6a); der Signatur-Index gilt dann unverändert.

## 13. Globale Constraints (Umsetzung)

- **Tests:** `cargo nextest run` only; Kommandos aus Repo-Root (kein `cd`,
  kein Shell-Chaining).
- **Vor jedem `git add`** (je Code-Datei): `cargo fmt`.
- **No worktrees** — direkt auf `feat-lmd-v2`.
- **Determinismus #498:** keine Timestamps/Counter/Random; embedded Seeds
  byte-identisch zur on-disk-Quelle.
- **Fidelity:** jede Original-Sektion landet in genau einem lmd-Ziel; verbatim
  portieren, nur Reference-Closure-Edits ändern.
- **Sprache:** Code/Kommentare Englisch; Interaktion/Commits Deutsch mit Umlauten.
- **`.lmd.md`-Rohquelle** nur via `git show HEAD:<path>` lesbar (Shadow-Hook rendert).
