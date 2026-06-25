---
title: lmd — native lean-ctx Live-Markdown-Engine v2.0
slug: lmd-lean-ctx-native-v2
status: draft
date: 2026-06-22
supersedes: docs/lean-md/specs/2026-06-15-lmd-lean-ctx-native-design-v1.md
consumer: ai
note: >
  Nachfolger der v1.0-Spec (2026-06-15). Reset auf den verifizierten Code-Stand:
  die Phasen 0–6 sind vollständig implementiert (`rust/src/lmd/` — 26 registrierte
  Bridges + `macros.rs`/`phases.rs`/Extension-Anbindung). v2 trägt den
  abgeschlossenen Stand nur noch als kompakten Status (§2) und richtet die
  Detailbeschreibungen auf die **offene** Arbeit aus (Phase 7–11, §5). Die
  lasttragenden Architektur-Invarianten (Bridge-Trait, Zwei-Klassen-Backing,
  Read→Delta, Dispatch-Enforcement-Kette, Security-Erbe, Single-Source-of-Truth)
  bleiben (§3), weil Phase 7–11 darauf aufsetzen. Das v1-Audit (CHANGELOG
  3.7.4 → 3.8.5, Greenfield-Inventur) ist erledigter Kontext und wandert **nicht**
  mit — es lebt weiter in v1 (superseded) und im Code.
---

# lmd — native lean-ctx Live-Markdown-Engine v2.0

> lmd ist **kein** externer Konsument von lean-ctx — lmd **erweitert** lean-ctx.
> Die nativen Direktiven sind der lean-ctx/lmd-Core. Hauptziel: kurze, aber sichere
> Pläne/Skills, die strukturell **nur** lean-ctx-native Tools adressieren, plus
> Subagent-Dispatch, der die Tool-Disziplin **erzwingt** statt sie zu empfehlen.

> **Stand 2026-06-22:** Phasen **0–6 implementiert & verifiziert** (§2). Die verbleibende
> v2-Arbeit ist **Phase 7–11** (§5). Diese Spec beschreibt den fertigen Stand kompakt und die
> offene Arbeit im Detail — „nur noch die nötigen Inhalte".

---

# Note für *superpowers:writing-plans

Bei jeder SDD-/Dispatch-Ausführung gilt die `.claude/rules/subagent-multi-agent.md` als
verbindlicher Contract (Controller **und** jeder Subagent). Im Plan festhalten.

## 1. Ziel & Abgrenzung

lmd baut die in `mdai/` (Node-markdownai) bewiesene Skill-/Plan-Mechanik nativ in lean-ctx
nach (rushdown als Parser) und verankert drei Dinge strukturell:

1. **lean-ctx als oberste Direktive** — kein Sprachkonstrukt für rohes `cat`/`grep`/`ls`, **kein**
   Verweis auf externe MCP-Tools (Serena, offizielles JetBrains-MCP — als Backings entfernt). Alle
   Code-Intelligenz läuft über lean-ctx-native Tools (`ctx_refactor`/`ctx_symbol`/`ctx_outline`/
   `ctx_graph`-Familie).
2. **TDD-Potenzial** (Token Dense Dialect + Tool-Routing): Pläne adressieren ab Geburt lean-ctx-Tools.
3. **Subagent-Dispatch ohne Drift** — der beobachtete Fehler (Subagents nutzen native bash/Read) wird
   an der Dispatch-Grenze + per Hook geschlossen (§3.4, scharf in Phase 7).

**Strangler, kein Big-Bang:** lmd wird der native Pfad (`ctx_md_*`-MCP-Tools + `lean-ctx md`-CLI,
Phase 9). Node-markdownai bleibt übergangsweise für nicht-migrierte `.mdai.md`. Voraussetzung jeder
Skill-Migration: der Phase-Isolations-Benchmark ist in lmd reproduziert, **bevor** ein Skill umzieht.

---

## 2. Verifizierter IST-Stand — Phasen 0–6 implementiert

Verifiziert gegen `rust/src/lmd/` (2026-06-22). Module: `args` `audit` `bridges/` `engine`
`fragments` `header` `macros` `node` `parser/{block,inline,mod}` `phases` `render`.
**26 Bridges** in `bridges/mod.rs::default_registry()` (Coverage-Guard
`default_registry_has_all_core_bridges`).

| Phase   | Inhalt                                                                                                  | Code-Ort (verifiziert)                                                            | Status |
|---------|--------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------|--------|
| **0**   | Audit (22 Einträge, CI-Guards) + rushdown-0.18-Spike                                                    | `audit.rs`                                                                         | ✅      |
| **1**   | `@lean-md`-Header, Block-/Inline-Parser, Bridge-Registry, Fragment-Resolver (built-in-first), geteilter `EngineContext`-Cache (Read→Delta ohne `fresh`); `@read`/`@include` | `header.rs` `parser/` `fragments.rs` `engine.rs` `bridges/{read,include}.rs`       | ✅      |
| **2**   | R-Bridges `@search`/`@list`/`@env`/`@date`/`@count`/`@query` (consumer-gegatet, §6-Defenses)            | `bridges/{search,list,env,date,count,query}.rs`                                    | ✅      |
| **3**   | `@graph` — 7 statische Ops (`dependents`/`dependencies`/`related`/`callers`/`callees`/`context`/`recent-neighbors`), read-only, **kein LSP** | `bridges/graph.rs`                                                                 | ✅      |
| **3.1** | `@edit` (text → `ctx_edit`; symbolisch → `ctx_refactor replace_symbol_body`/`insert_*`, BLAKE3-Guard)   | `bridges/edit.rs` (+ `bridges/addressing.rs` `name_path`)                          | ✅      |
| **3.2** | `@symbol` (`refs`/`def`/`impl`/`declaration`/`type-hierarchy`/`overview`) + Cache-Namen-Anreicherung    | `bridges/symbol.rs`                                                                | ✅      |
| **3.3** | `@refactor` (`rename`/`move`/`safe-delete`/`inline`, 2-Phasen `plan_hash`) — IDE-only                   | `bridges/refactor.rs`                                                              | ✅      |
| **3.4** | `@reformat` + `@inspect` (`ctx_refactor reformat`/`inspections`) — IDE-only                             | `bridges/{reformat,inspect}.rs`                                                    | ✅      |
| **3.5** | `@find` (`ctx_semantic_search`) + `@repomap`/`@impact`/`@architecture`/`@outline`                       | `bridges/{find,repomap,impact,architecture,outline}.rs`                            | ✅      |
| **3.6** | Quality `@smells`/`@review`/`@routes`                                                                   | `bridges/{smells,review,routes}.rs`                                               | ✅      |
| **4**   | E-Konstrukte: `@define`/`@call`, `@import`, `@if`/`@consumer`, `{{ expr }}`, Pipe + `@render` (+ `evalexpr`) | `macros.rs` (`MacroRegistry`/`extract_definitions`/`prune_containers`/`eval_*`) + `bridges/{call,render}.rs` | ✅      |
| **5**   | Extension-Anbindung (Achse B): `@call <plugin_tool>` (Plugin-`[[tools]]`, `extensions=allow`-Gate) + `@render type=<custom>` (`RenderTransform` via `core::extension_registry`) | `bridges/call.rs` (`resolve_plugin_call`/`PluginManager`) + `bridges/render.rs` (`RenderTransform`) | ✅      |
| **6**   | `@phase`/`@on complete` (Lifecycle → `session_decision`/`auto_findings`) + `@remember`/`@recall`        | `phases.rs` (`render_with_phases`/`fire_action`) + `bridges/{remember,recall}.rs` | ✅      |

**Aufgelöste Altlasten (v1):** R-1 (rushdown 0.18), G-1 (`recent-neighbors`), F-1/F-2 (Read-Delta /
Comment-Injection), Q-06 (JetBrains/PSI-Pfad — `ctx_refactor` bündelt die volle Fläche, #413).

---

## 3. Bleibende Architektur-Invarianten (tragen Phase 7–11)

Diese Entscheidungen sind durch die Phasen 0–6 etabliert und **gelten unverändert weiter** — die
offene Arbeit setzt darauf auf. Hier nur, was 7–11 braucht; die Herleitung steht in v1 (superseded).

### 3.1 Bridge-Trait + ~6 Engine-Primitive — neue Fläche kostet null Primitive

Jede Router-Direktive ist eine **Bridge** (dünner Adapter in eine bestehende lean-ctx-Core-API), kein
neues Engine-Primitiv. Das Interface (`bridges/mod.rs`):

```rust
pub trait DirectiveBridge {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError>;
    fn accepts_pipe(&self) -> bool { false }   // Pipe-Konsum (§4-Render), default aus
}
```

Die **~6 E-Primitive** (rushdown-Konstrukte ohne lean-ctx-Äquivalent) sind gebaut und abgeschlossen:
Block-Parser, Inline-Parser, Container-Transformer (`@if`/`@consumer`), Macro-Engine (`@define`/
`@call`/`@import`), Pipe+`@render`, TDD-Render-Hook (Primitiv #6 ist Phase 8 — der **einzige** noch
offene E-Punkt). **Konsequenz für Phase 7:** `@dispatch`/`@handoff` sind **Bridges** (R bzw. R+H) —
sie kosten **kein** neues E-Primitiv. `@dispatch` braucht lediglich etwas Core-Code (Pre-Pass +
`EngineContext`-Feld + Template-Splice, §5.1), aber **keine** neue Container-/Engine-Klasse.

### 3.2 Zwei-Klassen-Backing-Regel (headless vs. IDE-only) — Test-relevant

`ctx_refactor` degradiert über 3 Backings (#413, Journey 19 §1.1). Direktiven zerfallen in zwei
Klassen — verbindlich für Golden-Parity (§8):

| Klasse               | Direktiven/Ops                                                                                                                       | Backing ohne IDE                                | CI/Golden          |
|----------------------|-------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------|--------------------|
| **Headless-fähig**   | `@symbol refs/def/impl/overview`, `@edit` (text + symbolische body-edits), `@find`, `@repomap`, `@impact`, `@architecture`, `@outline`, `@smells`, `@review`, `@routes`, `@graph`, `@handoff` | rust-analyzer / tree-sitter / Property-Graph     | ✅ deterministisch  |
| **IDE-erforderlich** | `@symbol declaration/type-hierarchy`, `@refactor`, `@reformat`, `@inspect`                                                           | `BACKEND_REQUIRED` / „requires JetBrains backend" | ❌ eigener Test-Pfad |

**Design-Regel:** IDE-erforderliche Direktiven sind **nie** ein harter Engine-Gate (kein erzwungenes
`@reformat` im Render-Pfad) — sie liefern bei fehlender IDE eine klare Degradations-Meldung (Journey 19
§9). Headless-/CI-Renders brechen nie.

### 3.3 Read→Delta-Garantie + Cache-Kohärenz bei Writes

Geteilter `EngineContext`-Cache: `@read x` zweimal → 1. Full, 2. Cache-Hit/Delta **ohne `fresh`/`raw`**
(Anti-Pattern verboten, Engine + Dispatch-Constraint). Nach jedem `@edit`/`@refactor`/`@reformat`
evictet lean-ctx die Datei; der nächste `@read` re-validiert per mtime (~13 tok); Multi-File-Refactor
mtime-checkt jeden `changed_path` (Journey 19 §7.4). **Phase-7-Relevanz:** Der dispatchte Subagent erbt
diesen warmen Shared-Cache (ein MCP-Prozess) — seine Work-Bridges laufen ohne Re-Read-Kosten (D-3 lazy).

### 3.4 Dispatch-Enforcement-Kette (das eigentliche TDD-Ziel — Phase 7)

Drei Schichten, die die Tool-Disziplin **erzwingen** statt empfehlen:

1. **Quelle (lmd-Skill/-Plan):** I/O nur über native Direktiven — es gibt kein `cat` und keinen
   `serena`/`jetbrains`-Macro-Call zu schreiben.
2. **Dispatch (`@dispatch`, reiner Prompt-Renderer — kein Spawn):** generierter Subagent-Prompt aus
   (a) phasen-isoliertem, TDD-komprimiertem Inhalt **+** (b) Tool-Disziplin-Contract (Reads ohne
   `fresh`/`raw`; **kein** `fresh` nach Cache-Read; **keine** externen MCP-Tools — nur `ctx_*`) **+**
   (c) `ToolSearch(select:mcp__lean-ctx__ctx_*)`-Bootstrap (lädt deferred lazy-core **vor** dem ersten
   Read).
3. **Backstop (Hooks):** bestehende Deny-Hooks (`hook_handlers/mod.rs`); Read-Deny hat **kein** „Edit
   braucht native Read"-Schlupfloch mehr (`@edit`→`ctx_edit`); PreToolUse-Greifen im Subagent-Loop
   verifizieren.

Schicht 2+3 sind der Phase-7-Liefergegenstand (§5.1). Schicht 1 ist durch die native Direktiven-Fläche
(Phase 0–6) bereits Realität.

### 3.5 Single Source of Truth für die `ctx_refactor`-Fläche

lmd **friert keine `ctx_refactor`-Signaturen ein** (sie driften). Maßgeblich: `docs/reference/
19-jetbrains-plugin.md` (Journey 19) + `rust/src/tools/registered/ctx_refactor.rs` (`tool_def()`) +
`rust/src/lsp/router.rs` (`select_backend`). Pro Bridge bleibt nur: die **Backing-Klasse** (§3.2) +
der passende Degradations-Envelope. Tool-Params/Signaturen → `docs/reference/appendix-mcp-tools.md`.

---

## 4. Direktiven-Fläche (Referenz — implementiert ✅ / offen ◻)

Vollständige Adress-Fläche. Implementierte Direktiven kompakt; die offenen tragen ihre
Detailbeschreibung in §5.

| Direktive                          | Klasse | Backing (lean-ctx-nativ)                                                            | Status        |
|------------------------------------|--------|-------------------------------------------------------------------------------------|---------------|
| `@read`                            | R      | `core::structured_read` / `ctx_read`                                                | ✅ P1          |
| `@search` / `@list`                | R      | `ctx_search` / `ctx_tree`                                                            | ✅ P2          |
| `@query` (shell)                   | R      | `shell/exec` + compress (consumer-Gate `shell=allow`, §6)                            | ✅ P2          |
| `@env` / `@date` / `@count`        | R      | `std::env` / chrono / glob                                                           | ✅ P2          |
| `@graph`                           | R      | `graph_index`/`call_graph`/`graph_context` (statisch, kein LSP)                      | ✅ P3          |
| `@edit`                            | R      | `ctx_edit` (text) **+** `ctx_refactor replace_symbol_body`/`insert_*` (symbolisch)  | ✅ P3.1        |
| `@symbol`                          | R      | `ctx_refactor` (nav) + `ctx_symbol`/`ctx_outline` + Cache-Namen-Anreicherung         | ✅ P3.2        |
| `@refactor`                        | R      | `ctx_refactor rename/move/safe_delete/inline` (2-Phasen, `plan_hash`) — IDE-only    | ✅ P3.3        |
| `@reformat` / `@inspect`           | R      | `ctx_refactor reformat`/`inspections` — IDE-only                                    | ✅ P3.4        |
| `@find`                            | R      | `ctx_semantic_search` (BM25/dense/hybrid)                                            | ✅ P3.5        |
| `@repomap`/`@impact`/`@architecture`/`@outline` | R | `ctx_repomap`/`ctx_impact`/`ctx_architecture`/`ctx_outline`                     | ✅ P3.5        |
| `@smells` / `@review` / `@routes`  | R      | `ctx_smells` / `ctx_review` / `ctx_routes`                                           | ✅ P3.6        |
| `@define`/`@call`/`@import`/`@if`/`@consumer`/`{{ }}`/Pipe | E | Macro-Engine + Container-Transformer (`macros.rs`)                          | ✅ P4          |
| `@call <plugin_tool>` / `@render type=<custom>` | R | `core::plugins`/`core::extension_registry` (WASM/Plugin, `extensions=allow`)    | ✅ P5          |
| `@phase`/`@on complete`            | R+H    | `session_decision` / `core::auto_findings::extract`                                 | ✅ P6          |
| `@remember` / `@recall`            | R      | `ctx_knowledge`                                                                      | ✅ P6          |
| `@lean-md` Header                  | E      | Config-Parse                                                                         | ✅ P1          |
| **`@dispatch`**                    | R+H    | Fragment-Komposition (`fragments.rs`) + `capture_phase_bodies`-Pre-Pass + `EngineContext.phase_bodies`-Lookup + `splice_template_only`. **Reiner Prompt-Renderer, kein Spawn** | ◻ **P7** (§5.1) |
| **`@handoff`**                     | R      | `ctx_handoff` (Context Ledger Protocol, deterministic bundles)                      | ◻ **P7** (§5.1) |
| **TDD-Output**                     | R+E    | `tdd_schema` (R) + Render-Hook (E) + self-describing Legend (#580)                  | ✅ P8 — implemented (Phase 8) |

---

## 5. Offene Arbeit — Phase 7–11 (Detailbeschreibungen)

### 5.1 Phase 7 — `@dispatch` + `@handoff` (Dispatch & Hand-over)

> **Maßgebliches Design:** `docs/lean-md/specs/2026-06-21-lmd-phase-7-dispatch-handover-design.md`
> (Scope-Entscheidungen D-1…D-13, Architektur, Test-/Parity-Strategie, Implementierungs-Reihenfolge).
> Dieser Abschnitt fasst nur die tragenden Entscheidungen zusammen.

Zwei Bridges in `bridges/` (neben `remember.rs`/`recall.rs`), **null** neue E-Primitive:

| Direktive                       | Datei                 | Backing / Mechanik                                                                                                                                                  | Klasse |
|---------------------------------|-----------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|
| `@dispatch phase="…" role=…`    | `bridges/dispatch.rs` | Fragment-Komposition (`fragments.rs` `dispatch-contract`-`const`) + `capture_phase_bodies`-Pre-Pass + `EngineContext.phase_bodies`-Lookup + `splice_template_only`  | R+H    |
| `@handoff create\|show\|pull`   | `bridges/handoff.rs`  | `ctx_handoff` (Context Ledger Protocol)                                                                                                                             | R      |

**Form & Semantik:**

```text
@phase "A3-parser"                       # Phase 6 — definiert den isolierbaren Scope
  @read src/parser/block.rs
  @edit symbol=parse old=… new=…
  @query "cargo nextest run"
@phase-end
@dispatch phase="A3-parser" role=dev to_agent="{{ controller_id }}"
```

→ rendert einen vollständigen Subagent-Prompt aus **(a) isoliertem `@phase`-Inhalt + (b) Dispatch-
Contract + (c) `ToolSearch`-Bootstrap**.

**Tragende Scope-Entscheidungen:**

- **`@dispatch` = reiner Prompt-Renderer (D-1)** — **kein** Agent-Spawn. Der lean-ctx-MCP-Server kann
  keinen Claude-Subagenten starten; das kann nur die Harness (`Task`/`Agent`). `@dispatch` gibt nur
  Markdown zurück; der Controller übergibt ihn selbst an `Task`. Hält render-only (§3.1) +
  Determinismus (#498 — ein Spawn wäre non-deterministisch).
- **Template eager, Work lazy (D-3):** die Makro-/Container-Schicht des Phase-Bodys (`@call`/`@import`/
  `@define`/`@include`/`{{ }}`/`@if`/`@consumer`) löst **beim Dispatch** auf (nur der Controller kennt
  die Bindings: Task-Name, Ziel-Datei, Controller-ID); **Work-Bridges** (`@read`/`@search`/`@edit`/
  `@symbol`/`@graph`/`@query`) gehen **verbatim** in den Prompt — der Subagent führt sie in *seinem*
  warmen Shared-Cache aus (§3.3). Eager-Rendern der Work-Bridges zerstörte den Token-Hebel (−88…−95 %).
- **Phase-Isolation via Pre-Pass, nicht Funktions-Helper (D-4 — korrigiert ggü. Brainstorming):**
  `@dispatch phase="name"` schlägt den isolierten Body über das **neue Feld `EngineContext.phase_bodies`**
  nach, das ein **render-/lifecycle-freier `capture_phase_bodies`-Pre-Pass** (in `render_body` nach
  `prune_containers` verdrahtet) ROH befüllt — `phase_body(name)`-Lookup, analog `@define`→`ctx.macros`.
  Grund: Der Splice-Kontext sieht den umgebenden `@phase` **nicht**. Das vermutete `ctx_md_read_phase`
  existiert **nicht**; `phases.rs::render_with_phases` ist ein render-time Line-Scanner mit Lifecycle-
  Side-Effects (`session_decision`/`@on complete`/`finalize_phase`) und bietet **keine** isolierte
  Body-Extraktion. `splice_template_only` (`render.rs`) splict wie `splice_directives`, hält aber
  `WORK_DIRECTIVES` + Pipes verbatim und löst nur `@call`/`@include`/`{{ }}` eager auf.
- **Contract-Quelle (D-5/D-8/D-11):** Block (b) ist ein **built-in `const DISPATCH_CONTRACT`**
  (`fragments.rs`, portiert aus `.claude/rules/subagent-multi-agent.md`, `{{ role }}`/
  `{{ controller_id }}`-parametrisiert), via `.lmd.md` overridable. Das veraltete built-in `hard-rules`
  (nennt entfernte serena/jetbrains-Backings) wird **inline im `const` gefixt** (→ `ctx_refactor`/
  `@symbol`/`@edit`). **Kein `include_str!`, keine `lean-md/`-Datei** in Phase 7 → keine Build-/Datei-
  Kopplung, kein Stub, Binary-embedded/headless garantiert (läuft **nicht** über `auto_inject_rules`).
  Die `lean-md/core/`-Datei-Anlage + `include_str!`-Migration ist **Phase 10** (§5.4).
- **`@handoff` orthogonal, kein Auto-Wiring (D-1):** `@dispatch` ruft `ctx_handoff` **nicht** automatisch.
  `@handoff` ist eine **explizite** Direktive für durable Bundles. Abgrenzung: `ctx_handoff` =
  datei-artiges, `pull`-bares Bundle; `ctx_agent action=handoff` = leichtgewichtiger Live-Bus-Baton
  (Instruktions-Text im Contract, keine eigene Direktive). v1-Aktions-Set `@handoff` = `create|show|pull`
  (`list|export|import` deferred).

**Hook-Verifikation (D-10):** Kein neuer Hook-Code. Die Infra (`hook_handlers/mod.rs`) interceptet
PreToolUse für Read/Grep/Edit/Shell; das „Edit braucht native Read"-Schlupfloch ist via `@edit→ctx_edit`
bereits zu. Phase 7 liefert **nur** den Dispatch-Drift-Test (§8): dispatchter Subagent → **null** native
`Read`/`cat`-Calls + **null** externe MCP-Calls (Hook-Deny-Zähler == 0), inkl. Subagent-Loop-Greifen.
Greift der Hook im Task-Loop *nicht* → dokumentierter Follow-up (Claude-Code-Settings), **nicht** in
diesem Schnitt nachgebaut.

### 5.2 Phase 8 — TDD-Render-Hook

Letztes offenes E-Primitiv (§3.1 #6). Renderer-Ausgang hängt an `tdd_schema` (Modi `tdd`/`compact`/
`off` aus dem `@lean-md`-Header) + erbt die self-describing Legend (#580, ≤15 tok). R-Teil = `tdd_schema`,
E-Teil = Render-Hook am Renderer-Ausgang. Gate: Output-Kompression messbar, Legend byte-stabil (#498).

### 5.3 Phase 9 — `ctx_md_*`-MCP-Tools + `lean-ctx md`-CLI (Strangler-Oberfläche) ✅ implementiert

Exponiert die Engine als MCP-Tools (`ctx_md_render`/`ctx_md_check`) + CLI (`lean-ctx md render|check`).
`consumer=human` narriert Direktiven als Prosa (lesbare Plan-Darstellung). Aktualisiert
`appendix-mcp-tools`/`appendix-cli-map`/Profil-Tabellen (Tool-Count 77→79). MCP-Envelope-Guard:
`{content, structuredContent}`, Failures setzen `isError` (3.8.0). Design:
`docs/lean-md/specs/2026-06-22-lmd-phase-9-human-render-design.md`. **Hinweis:** ein
`ctx_md_read_phase` wird hier **nicht** vorausgesetzt — die Phase-Isolation lebt in
`@dispatch`/`capture_phase_bodies` (§5.1, D-4).

### 5.4 Phase 10 — Layout & Verfügbarkeit (Infrastruktur)

Phase 10 ist **reine Infrastruktur** — **kein** Skill-Inhalt (der ist Phase 11, §5.5). Sie friert das
`lean-md/`-Layout ein, schließt die `const`→`include_str!`-Contract-Migration ab und schafft die
**Verfügbarkeit** (Opt-in-Install-Pfad) für lmd-Skills bei **binary-only-Auslieferung**. Ziel: die
Infrastruktur, die das Authoring des ersten Skills (Phase 11) trägt und den Token-Hebel strukturell
garantiert.

**Zwei Schichten — Seed vs. Runtime (binary-only).** `lean-md/` ist der **Quell-/Seed-Baum** (komplett
via `include_str!` ins Binary kompiliert), **kein** Laufzeit-Pfad:

```text
lean-md/                               # Quell-Seeds → ins Binary embedded. KEIN Runtime-Pfad.
├── core/                              # KERN — embedded, locked, von @dispatch immer vorangestellt
│   ├── hard-rules.lmd.md              #   KANON (serena/jetbrains → ctx_refactor)
│   ├── dispatch-contract.lmd.md       #   ← Migration aus const DISPATCH_CONTRACT (fragments.rs)
│   └── _fragments/                    #   geteilte Sub-Fragmente
├── templates/dispatch-contract.ext.lmd.md   # Erweiterungs-Seed → .lean-ctx/ materialisiert
├── lang/      rust.lmd.md             # Sprach-Pack-Seeds (cargo/nextest via @query)
├── tooling/                           # MCP-/Plugin-Tool-Pack-Seeds (NICHT serena/jetbrains)
└── skills/    lmd-brainstorm/         # Skill-Seeds (Pilot — Body in Phase 11 befüllt)
    ├── SKILL.md                       #   dünner Delegations-Stub (Frontmatter + ctx_md_render)
    └── body.lmd.md                    #   embedded; phasenweise via ctx_md_render(skill, phase)
```

**Runtime-Materialisierung** (wohin das Binary schreibt / woraus gelesen wird):

| Seed                        | Ziel zur Laufzeit                       | Scope            | Begründung                                  |
| `core/`                     | — bleibt im Binary, render-time gelesen | embedded/locked  | headless, byte-stabil, nie überschrieben    |
| `skills/<name>/SKILL.md`    | `~/.claude/skills/<name>/` (per Agent)  | **global**       | Harness-Skill-Discovery ist agent-global    |
| `skills/<name>/body.lmd.md` | — bleibt im Binary                      | embedded         | render-on-invoke → Phase-Isolation erhalten |
| `templates/*.ext.lmd.md`    | `.lean-ctx/lean-md/`                    | **Projekt**      | Erweiterung projekt-spezifisch              |
| `lang/*.lmd.md`             | `.lean-ctx/lean-md/lang/`               | **Projekt**      | Sprache unterscheidet je Projekt            |
| `tooling/*.lmd.md`          | `.lean-ctx/lean-md/tooling/`            | **Projekt**      | Tooling unterscheidet je Projekt            |

Auflösungs-Reihenfolge: Projekt `.lean-ctx/lean-md/…` **überschreibt** den embedded Seed; fehlt die
Projekt-Datei, greift der Seed. Der **global** installierte Skill rendert dennoch **projekt-aware**
(zieht beim Render die `lang/`/`tooling/`-Overrides aus dem `.lean-ctx/` des *aktuellen* Projekts).

**Contract-`const` → `include_str!`-Migration (Abschluss D-5/D-11, Phase-7-Versprechen):** der in Phase 7
binary-embedded `const DISPATCH_CONTRACT` (`fragments.rs`) + das inline gefixte built-in `hard-rules`
werden nach `lean-md/core/dispatch-contract.lmd.md` / `core/hard-rules.lmd.md` ausgelagert und via
`include_str!` eingebunden — keine Verhaltensänderung, nur Quell-Ort. Fragment-Konsistenz-Test (§8 #9)
garantiert Byte-Identität. `subagent-multi-agent.md` wird auf einen **Zeiger** auf den Contract-Kanon
reduziert (D-7) — nur noch *ein* Ort, Drift strukturell unmöglich. `@dispatch` komponiert **core-first
selbst** (`Kern + Erweiterung`), verlässt sich **nicht** auf ein User-`@include` des Kerns.

**Verfügbarkeit — Skill-Install bei binary-only (Opt-in).** Der lmd-Skill reitet auf demselben
Setup-Zeit-Materialisierungs-Pfad wie der `lean-ctx`-Skill (`rules_inject::install_all_skills` →
`<agent>/skills/<name>/SKILL.md`, `include_str!`-embedded, atomic, idempotent), mit **einem**
Polaritäts-Delta:

|                             | `lean-ctx`-Skill                        | lmd-Skill                                                              |
| Install-Trigger             | `auto_inject_skills` (auto, always-on)  | **nur** `lean-ctx config set lean-md skill true`                      |
| Auto-Inject-Default greift? | ja                                      | **nein** — eigenständiger, entkoppelter `install_lmd_skills`-Pfad     |
| Gate                        | opt-out (`rules_injection=off`)         | **opt-in** (`[lean-md] skill`, default `false`) + erbt `=off`         |
| SKILL.md-Inhalt             | statisch (alles inline)                 | **dünn**: Frontmatter + render-on-invoke via `ctx_md_render(skill, phase)` |

`config set lean-md skill true` → schreibt Config **+** ruft `install_lmd_skills(home)` (schreibt
`~/.claude/skills/lmd-brainstorm/SKILL.md` pro erkanntem Agent) → User startet **neue** Session → Harness
entdeckt den Skill nativ. **Kein** In-Session-Restart-Handling (Install passiert per CLI *vor*
Session-Start). `false` spiegelt `remove_claude_skill` (entfernt **nur** das lmd-eigene Dir).
**Render-Surface:** `ctx_md_render` (MCP, Phase 9) bekommt `skill=<name>`/`phase=<name>`-Adressierung
gegen den embedded Body; **kein** `lean-ctx md render`-CLI-Pfad im Stub (bewusst nicht weiterverfolgt).

**`[lean-md]`-Config-Erweiterung:**

```toml
[lean-md]
skill = false                  # opt-in master-gate (deny-by-default)
# skills = ["lmd-brainstorm"]  # optional selektiv; leer + true → alle bekannten lmd-Skills
contracts_dir = ".lean-ctx/lean-md"   # Projekt-Materialisierungs-Ziel (Schicht B)
materialize_contracts = false  # lang/tooling/.ext nach contracts_dir ziehen (nur wenn absent)
```

**Tool-Verfügbarkeits-Audit (Kernanliegen „alle Tools ausgenutzt"):** Coverage-Matrix
(Brainstorming-Workflow-Schritt → lmd-Direktive → lean-ctx-Backing) als prüfbares Artefakt + Gate (§8).
Jede benötigte Direktive ist in `default_registry()` registriert; Tools *ohne* Direktiven-Pendant werden
explizit als „bewusst nicht im Brainstorming-Pfad" gelistet (Transparenz statt stillem Loch).

**Layout-Konvention (der −95 %-Hebel, verbindlich dokumentiert):** Benchmark-Befund v5 #4 — Hard-Rules
**global** vor allen Phasen → flach −70 %; Hard-Rules in **einer** `pre-context`-Phase → bis −95 %
(`mdai-benchmark.md`, `handoff` = 291 Tok). Kanonisches Skill-Body-Skelett: schwerer `@include`-Block
(`core/hard-rules` + `dispatch-contract` + tool-quick-ref) **nur** in `pre-context`, Folge-Phasen
(… → `handoff`) lean. Dies ist das Phase-10-Deliverable, das den Token-Gewinn des Phase-11-Skills
strukturell garantiert.

**Drei Injektions-Schichten, getrennt** (unverändert): Setup-Zeit (`install_*_skills`/`auto_inject_skills`
— statische Dateien) · Hook-Zeit (PreToolUse-Redirect) · Render-Zeit (`@dispatch`-Contract,
Binary-embedded). Repo-Fetch über Netzwerk bleibt verworfen (bräche Headless/Determinismus).
**Verworfen (Achse A):** Cargo-Feature-Gating der lmd-Engine — spart bei single-binary-Auslieferung
keinen End-User-Platz und gatet die Phase-9-Tools (`ctx_md_*`) mit weg (§10).

**Implementierungsstand (✅ Phase 10 abgeschlossen):** Contract-Migration (`const`→`include_str!`)
abgeschlossen — `hard-rules.lmd.md` + `dispatch-contract.lmd.md` in `lean-md/core/`, byte-identisch
per `builtin_fragments_match_seed_files_on_disk`-Test verifiziert. `install_lmd_skills`-Pfad
entkoppelt von `auto_inject_skills` (opt-in `[lean-md] skill`). `ctx_md_render(skill, phase)`-
Adressierung live (`render_skill` + Phase-Isolation-Gate `lmd_phase10_gate`). Tool-Verfügbarkeits-Audit
(`availability::COVERAGE`, 11 Direktiven, 3 explizite Lücken) als Registry-Gate in `lmd_phase10_gate`.

### 5.5 Phase 11 — Pilot-Skill `lmd-brainstorm` authoren

Setzt auf der Phase-10-Infrastruktur auf — **Skill-Inhalt, kein Infra-Code**:

- **`body.lmd.md` schreiben:** der Superpowers-`brainstorming`-Skill in **Funktion/Reihenfolge/
  Komplexität** an lmd-Direktiven adaptiert — Phasen-Sequenz explore → questions → approaches →
  present-design → write-spec → self-review → handoff, nach der `pre-context`-Layout-Konvention (§5.4).
  Migration ggü. mdai-Vorlage: `body.mdai.md` → `body.lmd.md`; `mcp__markdownai__read_file` →
  `@dispatch phase=…` (Phase-Isolation via `capture_phase_bodies`/`phase_body` + `splice_template_only`);
  `@call ctx_read`/`ctx_tree` → native `@read`/`@list`; `@call find_symbol`/`replace_symbol_body` →
  native `@symbol`/`@edit`; Header `@markdownai v1.0` → `@lean-md`.
- **SKILL.md-Frontmatter** (`name`/`description`) finalisieren — den dünnen Phase-10-Stub mit Inhalt füllen.
- **Gates:** Golden-Output-Parity gegen Node-`mdai-brainstorm`-Render (gleicher sichtbarer Inhalt pro
  Phase) + **Phase-Isolation-Token-Check** (kleine Phase −88…−95 %, Marken aus `mdai-benchmark.md` v5:
  `handoff` ≈ 291 Tok, `pre-context` trägt den Regel-Block).
- **Akzeptanz:** erster Skill produktiv auf lmd, via `lean-ctx config set lean-md skill true`
  (deny-by-default) aktivierbar.

---

## 6. Security — erben, nicht neu erfinden

lmd-Direktiven routen durch lean-ctx-Tools und erben Defense-in-Depth on-by-default — kein eigener
Edit-/Refactor-/Cache-/Audit-Schutz. Für die offene Arbeit relevant:

- **`ctx_refactor`-Familie (Journey 19 §7–§8):** PathJail (jeder plugin-gemeldete `changed_path`
  re-validiert), Token-Auth pro Projekt (loopback-only), BLAKE3-Conflict-Guard (`expected_hash`/
  `plan_hash`, TOCTOU), Smart-Mode (`INDEXING` statt Teil-Resultat), Atomicity.
- **`@query`/`@call ctx_shell` (#391):** `shell_strict_mode` blockt Command-Substitution (`$(…)`/
  backticks) + Pipe-to-bare-Interpreter; consumer-gegatet (`@lean-md shell=allow`), Deny-by-Default-
  Allowlist (≈201 Binaries). Keine eigenen Deny-Patterns.
- **Extension-Exec-Gate (Phase 5):** `@call <plugin_tool>` hinter `@lean-md extensions=allow`
  (deny-by-default, analog `@query`); geerbte `SandboxPolicy` (env-scrub + cwd-jail + timeout).
  WASM-`@render type=<custom>` braucht **kein** Gate (empty-linker-Sandbox, deterministisch).
- **`@dispatch`/`@handoff` (Phase 7):** `@dispatch` rendert **nur** — kein Subprocess, kein Spawn, keine
  neue Angriffsfläche. Der isolierte Phase-Inhalt durchläuft dieselben Bridge-Gates wie beim Direkt-
  Render (`@query` consumer-gegatet, Work-Bridges erben PathJail). Fragment-Resolve erbt den Jail;
  `include_str!`-Built-ins sind compile-time. `@handoff` erbt `ctx_handoff`-Defense. Determinismus #498:
  Output ist deterministische Funktion aus (Phase-Inhalt, Contract-Bytes, Bootstrap, `role`/`to_agent`).
- **`@include`/`@import`-Kette:** `max_chain_depth=16`, keine Symlink-Eskalation.
- **`consumer: ai/human`** steuert **nur** die Render-Verdichtung (`@if consumer=human` ausblenden beim
  `ai`-Render), ist **kein** Privileg-/Zugriffsmodell — Schreibrechte (`@remember`) regeln Role-Policies.

---

## 7. Phasenplan

| Phase     | Inhalt                                                                                                       | Gate / Ergebnis                                                              |
|-----------|-------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------|
| **0–6**   | ✅ **implementiert** (§2) — Audit, Parser/Bridge-Registry/Cache, alle R-Bridges (`@read`…`@routes`), E-Konstrukte (`@define`/`@if`/`{{ }}`/Pipe), Extension-Anbindung (`@call`/`@render`), `@phase`/`@remember` | verifiziert gegen `rust/src/lmd/`                                            |
| **7**     | **`@dispatch`** (reiner Prompt-Renderer: isolierter Inhalt + Dispatch-Contract + ToolSearch-Bootstrap) + **`@handoff`** (`ctx_handoff`) + Tool-Disziplin-Constraint-Injektion + Hook-Verifikation. Contract als Rust-`const` (Binary-embedded, **nicht** via `auto_inject_rules`); `lean-md/`-Datei + `include_str!` → Phase 10. **Design `2026-06-21-…-phase-7-…md`** | Subagent-Dispatch ohne Drift                                                |
| **8**     | TDD-Render-Hook (`tdd_schema` + self-describing Legend #580)                                                 | Output-Kompression messbar                                                   |
| **9**     | ✅ **implementiert** — `ctx_md_render`/`ctx_md_check`-MCP-Tools + `lean-ctx md render|check`-CLI (+ appendix/Profil-Tabellen); `consumer=human`-Narrations-Render. Design: `docs/lean-md/specs/2026-06-22-lmd-phase-9-human-render-design.md` | Strangler-Oberfläche                                                         |
| **10**    | ✅ **implementiert** — **Layout & Verfügbarkeit (Infrastruktur)** — `lean-md/`-Layout (Seed vs. Runtime), `const`→`include_str!`-Contract-Migration (D-5/D-11), Opt-in-Skill-Install bei binary-only (`[lean-md] skill`, `install_lmd_skills` entkoppelt von auto-inject), `ctx_md_render(skill, phase)`-Adressierung, Tool-Verfügbarkeits-Audit, `pre-context`-Layout-Konvention | Infrastruktur trägt Phase 11; kein Skill-Inhalt |
| **11**    | **Pilot-Skill `lmd-brainstorm` authoren** — Superpowers-`brainstorming` (Funktion/Reihenfolge/Komplexität) an lmd-Direktiven adaptiert, `pre-context`-Layout; SKILL.md-Frontmatter finalisiert | erster Skill auf lmd: Golden-Parity + Phase-Isolation-Token-Marken (−88…−95 %) |

---

## 8. Test- / Parity-Strategie (offene Phasen)

1. **Golden-Output-Parity (Headless-Klasse):** jede headless-fähige Direktive/jeder Skill rendert
   byte-nah identisch zum Node-Output (Snapshot). Nur Headless-Klasse (§3.2).
2. **IDE-Backing-Test-Pfad (IDE-only):** `@refactor`/`@reformat`/`@inspect`/`@symbol type-hierarchy`
   gegen das laufende JetBrains-Plugin; ohne IDE exakte `BACKEND_REQUIRED`/`UNSUPPORTED_LANGUAGE`-
   Degradation (Journey 19 §9). Kein Golden-Snapshot.
3. **Phase-Isolation-Token-Check (P7):** `@dispatch phase=…` rendert via `capture_phase_bodies`-Pre-Pass
   + `phase_body(name)`-Lookup + `splice_template_only` nur isolierten Inhalt + Contract + Bootstrap,
   kein Cross-Phase-Leak; tiktoken-Re-Messung trifft die Benchmark-Marken (kleine Phase −88…−95 %,
   7-Subagent-Dispatch ≈9 585 Tok). Abweichung >10 % = Befund.
4. **Dispatch-Render-Parity (P7):** `@dispatch role=dev` byte-stabil = Contract + isolierter Inhalt +
   Bootstrap (Snapshot); `role=review` → register-Zeile + finding-Hinweis. Eager/Lazy-Grenze:
   Work-Bridges (`@read`/`@query`/…) im Output **verbatim**.
5. **Fehler-Envelopes (P7):** `phase` unbekannt → `PHASE_NOT_FOUND`; `to_agent` leer → sichtbarer
   Platzhalter + Warnung (kein Abbruch); `dispatch-contract` nicht resolvebar → `CONTRACT_UNAVAILABLE`.
6. **Bridge-Unit (P7):** `@handoff create` == `ctx_handoff create` (Aktions-Mapping `create|show|pull`).
7. **MCP-Envelope (P9, #389):** `ctx_md_*` liefern `{content, structuredContent}`; Failures setzen
   `isError`.
8. **Dispatch-Drift-Test (P7, §3.4):** dispatchter Subagent → **null** native `Read`/`bash cat`-Calls
   + **null** externe MCP-Calls (Hook-Deny-Zähler == 0), Subagent-Loop-Greifen verifiziert.
9. **Fragment-Konsistenz (P10, #498):** built-in `dispatch-contract`/`hard-rules` == `lean-md/core/`-
   Dateien (`include_str!`-Identität) nach der Migration.
10. **Skill-Install-Roundtrip (P10):** `config set lean-md skill true` → `~/.claude/skills/lmd-brainstorm/
    SKILL.md` existiert; `false` → entfernt; **nur** das lmd-eigene Dir berührt (analog
    `skill_install_then_remove_roundtrips`).
11. **Skill-Opt-in / nicht-auto-injiziert (P10):** ohne `[lean-md] skill=true` schreibt
    `install_all_skills` **kein** lmd-`SKILL.md` (Entkopplung vom always-on `auto_inject_skills`-Default);
    Default `false`; `rules_injection=off` erzwingt Entfernung.
12. **Tool-Verfügbarkeits-Audit-Gate (P10):** jede Brainstorming-Workflow-Direktive ist in
    `default_registry()` registriert; die Gap-Liste (Tools ohne Direktiven-Pendant) ist byte-stabil.
13. **`ctx_md_render` Skill/Phase-Adressierung (P10):** `ctx_md_render(skill, phase)` löst gegen den
    embedded Body auf (kein Cross-Phase-Leak); `[lean-md]`-Config (`skill`/`contracts_dir`/
    `materialize_contracts`) round-trip (analog `lean_md_formatters_parse`).
14. **Phase-Isolation-Token-Check (P11):** am echten `lmd-brainstorm`-Body — kleine Phase −88…−95 %
    (`handoff` ≈ 291 Tok), tiktoken-Re-Messung trifft die `mdai-benchmark.md`-v5-Marken; Abweichung
    >10 % = Befund. (Engine-seitiger P7-Token-Check, §8.3, bleibt; diese Messung ist skill-spezifisch.)
15. **Golden-Output-Parity `lmd-brainstorm` (P11):** gegen Node-`mdai-brainstorm`-Render, gleicher
    sichtbarer Inhalt pro Phase (Snapshot).
16. Tests via `cargo nextest run`.

---

## 9. Offene Punkte / deferred

| ID       | Frage                                                              | Status                                                                                                                                                                                                                                                       |
|----------|-------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Q-05     | `@phase`-Fehlerverhalten (abort vs. continue)                     | **deferred** — scharf in der `executing-plans`-Migration. Übergangs-Default (implementiert, `phases.rs`): Body sequentiell, Error als `decision`-Eintrag + stabiler Envelope, Render bricht nicht ab                                                          |
| Q-07     | `@symbol`/`@refactor` in Nicht-JVM-IDEs (RustRover/PyCharm)        | **bekannt (Journey 19 §2.2):** `type_hierarchy`/IDE-PSI-`symbols_overview` JVM-only; Rust/Python degradieren auf `ctx_outline`/`implementations`/`ctx_callgraph`. Direktiven liefern `UNSUPPORTED_LANGUAGE`-Envelope statt Teil-Resultat                       |
| Q-08     | ctxpkg-Wissens-Kreislauf für Subagent-Dispatch (`@dispatch context_pack=`) | **Follow-up (deferred)** — scharf in der `subagent-driven-development`/`executing-plans`-Migration, **nicht** Engine-v2. Ein optionaler `@dispatch context_pack=`-Bootstrap gäbe Subagenten Projekt-Orientierung **zusätzlich** zum erzwungenen Disziplin-Kern (§3.4 — **nicht** aufweichen). Kein eigenes Format — ctxpkg/`ctx_pack`/`ctx_knowledge` erben |
| Q-09     | Legacy-Discipline-Hooks (`skill-plan-injector.py`/`plan-discipline.py`) entfernen | **Deferred-Cleanup, post-Phase-11** — **nicht** in P10/P11. Der `skill-plan-injector.py`-PostToolUse-Hook injiziert den `DRIFT_BLOCK` in **`writing-plans`/`requesting-code-review`** (nicht `brainstorming`); ein Entfernen direkt nach P11 (`lmd-brainstorm`) risse ein Disziplin-Loch in den noch klassischen Flows. Fällig **gated** auf die jeweilige Skill-Migration (deren embedded Dispatch-Contract + Skill-Body den `DRIFT_BLOCK` dann strukturell trägt). Begründung deckt sich mit `docs/mdai/design-skill-integration.md` §7 (Option a, ersatzlos), aber pro-Skill statt pauschal |

---

## 10. Was wir bewusst NICHT bauen

- `@http`/`@db`/`@fetch` — kein neuer externer Code (`ctx_url_read`/`ctx_git_read` existieren als Tools;
  Direktiven-Wrapper deferred).
- Externe MCP-Backings (Serena, offizielles JetBrains-MCP) — **entfernt**, ersetzt durch `ctx_refactor`.
- **Eigene WASM-/Plugin-Runtime** — lmd erbt lean-ctx' Extension-Runtime (`wasm-abi-v1`, EPIC
  12.8/12.9/12.11). WASM **nicht** für die `{{ param }}`-Macro-Substitution (native rushdown, Phase 4),
  nur für Dritt-Transforms (Phase 5). `@render type=<custom>` = ABI-Erweiterung der bestehenden
  `wasm-abi-v1` (`RenderTransform` + `lctx_render`), **kein** zweiter Runtime.
- **`@dispatch`-Agent-Spawn** — verworfen (D-1): reiner Renderer, der Controller übergibt an `Task`.
- **Auto-Wiring `@dispatch`→`ctx_handoff`-Bundle** — verworfen (D-1, Seiteneffekt).
- **Inline-Body-`@dispatch`** — verworfen (D-4, YAGNI, zweiter Container-Typ).
- **Neuer Hook-/Deny-Code (P7)** — Verifikation-only; Infra existiert, Schlupfloch via `@edit→ctx_edit` zu.
- **`@handoff list|export|import`** — Tool kann es, Direktive deferred (v2 schlank).
- **Contract-Distribution via `auto_inject_rules`** — bewusst nicht (D-12); Binary-embedded, headless.
- **Cargo-Feature-Gating der lmd-Engine (P10, Achse A)** — verworfen: spart bei single-binary-Auslieferung
  keinen End-User-Platz (eine Binary mit Feature-on wird ausgeliefert) und gatet die bereits ausgelieferten
  Phase-9-Tools (`ctx_md_*`) mit weg. Skill-Wahl läuft stattdessen über das Config-Opt-in (`[lean-md] skill`,
  §5.4) — Install-time, nicht Compile-time.
- **`lean-ctx md render`-CLI-Pfad im Skill-Stub (P10)** — bewusst nicht weiterverfolgt; render-on-invoke
  läuft über das MCP-Tool `ctx_md_render(skill, phase)`.
- **Legacy-Discipline-Hook-Removal in P10/P11** — nicht jetzt; gated auf die jeweilige Skill-Migration
  (Q-09, §9).
- Eigene Edit-/Refactor-Schutzschicht, eigene Cache-Schicht, eigener Audit-Log, eigenes Wissens-/Pack-/
  Distributions-Format — alles geerbt (`ctx_refactor`-Guards / Session-mtime-Cache / `audit_trail.rs` /
  ctxpkg).
- Custom `@consumer=*`-Audiences (nur `ai`/`human`); Parser pro Direktive (ein Block- + ein Inline-Parser);
  symbolisches `@reformat` als Pflicht-Gate (Konvention, IDE-only).

---

*Status: v2.0-draft (2026-06-22). Nachfolger von `2026-06-15-lmd-lean-ctx-native-design-v1.md` (v1.0,
superseded). Reset auf verifizierten Code-Stand: Phasen 0–6 implementiert (§2, `rust/src/lmd/` — 26
Bridges + `macros.rs`/`phases.rs`/Extension-Anbindung), nur noch als kompakter Status getragen;
Detailbeschreibungen gelten der offenen Arbeit Phase 7–11 (§5). Architektur-Invarianten (§3) unverändert.
Phase-7-Detail in `2026-06-21-lmd-phase-7-dispatch-handover-design.md`, Phase-5-Detail in
`2026-06-21-lmd-phase-5-extension-runtime-design.md`. Gebunden an `rust/src/lmd/bridges/mod.rs`
(`default_registry`), `macros.rs`, `phases.rs`, `bridges/{call,render}.rs`, Journey 19.*
