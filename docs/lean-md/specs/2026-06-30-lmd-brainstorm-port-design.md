# lmd-brainstorm — Voll-Port von superpowers:brainstorming (Design Spec)

**Datum:** 2026-06-30
**Branch:** `feat-lmd-v2` (direkt, keine Worktrees)
**Status:** Design — bereit für `writing-plans`

> **Supersedet:** `docs/lean-md/specs/2026-06-29-lmd-brainstorm-design.md`. Jener
> Entwurf war substanziell tragfähig (8 Phasen, Spec-Reviewer, Visual-Companion),
> entstand aber **vor** `lmd-test-driven-development` + `lmd-writing-skills`. Diese
> Schwester-Skills haben die jetzt **realisierten** Konventionen etabliert; dieses
> Spec biegt den Port darauf um. Drei Punkte des alten Specs sind dadurch
> erledigt/obsolet: (a) eigener `lean-md skill install`-Subcommand **existiert
> bereits** (`src/skill_install.rs`); (b) Body-Override (altes D7) ist Realität;
> (c) manuelles `@include dispatch-contract` entfällt — `@dispatch` prependet den
> Contract automatisch (Contract-Kanon D-7).

## Ziel

`superpowers:brainstorming` **vollständig und verlustfrei** als nativer lean-md-Skill
`lmd-brainstorm` adaptieren. Übergeordnetes Ziel: lean-md trägt die Skill eigenständig
— **superpowers wird für sie entbehrlich** (Reference-Closure: alle Querverweise zeigen
auf lmd-/lean-md-Ziele, nie zurück nach superpowers).

Das Muster ist die Schwester-Skill `lmd-writing-skills`
(`content/skills/lmd-writing-skills/`): dünner `SKILL.md`-Discovery-Stub + schwerer,
phasen-isolierter `body.lmd.md`, geteilte `@include`-Fragmente in `_includes/`,
Companions in `companions/`, install-materialisierte Assets — alles
`include_str!`-embedded, gerendert über `ctx_md_render(skill, phase|companion)`.

**Meta-Disziplin (Auftrag des Nutzers):** Beim Bauen dieser Skill wird
`lmd-writing-skills` **angewandt** — Iron Law **„NO SKILL WITHOUT A FAILING TEST
FIRST"**. Vor der Implementierung steht ein RED-Baseline-Pressure-Test (Agent **ohne**
die Skill brainstormt undiszipliniert → Rationalisierungen verbatim dokumentieren),
danach GREEN (mit gerenderter Skill compliant).

## Leitprinzipien

1. **Fidelity (kein Verlust).** Jede Sektion, Tabelle, jeder Checklist-Punkt, jedes
   Flowchart und jede Begleitdatei des Originals wird verbatim oder treu in genau
   ein lmd-Ziel (Phase / Companion / Asset) überführt. Nichts wird
   „wegzusammengefasst" oder gedroppt.
2. **lean-md-Mechanik nutzen.** Original eng spiegeln, aber Phasen-Rendering,
   Companions und Asset-Materialisierung von lean-md statt einer flachen
   Monolith-Datei.
3. **Reference-Closure.** Alle Querverweise auf lmd-/lean-md-Ziele umbiegen.
4. **Determinismus (#498).** Tool-Output ist deterministische Funktion von (Inhalt,
   Mode, CRP, Task) — keine Timestamps/Counter/Random. Embedded Seeds sind
   byte-identisch zur On-Disk-Quelle (Fragment-/Companion-Consistency-Gate).
   CLI==MCP byte-identisch.
5. **Harness-Agnostik.** Der Skill-Inhalt ist nicht Claude-spezifisch; über
   `ctx_md_render` ist er für **jeden** MCP-Agenten erreichbar. **Offen bleibt
   bewusst, welche Harness genutzt wird** — wir hardcoden kein Claude.

## Kein Engine-Change

Wichtige Abgrenzung: `lmd-brainstorm` benötigt **keinen** Eingriff in `engine.rs`,
`rushdown`/`evalexpr` oder die Bridges. Die einzige für diesen Skill nötige
Dispatch-Mechanik — `@dispatch` mit `skill=`+`companion=`-Brief-Quelle und Rolle
`review` — ist bereits geshipped (Commit `4c71084`, Spec
`2026-06-29-companion-split-dispatch-design.md`). Dieser Port ist damit reine
**Seed- + Registry- + `skill_install`-Asset-Arbeit**.

## Architektur

Ablage folgt der etablierten Schwester-Skill-Konvention: Companions in
`companions/`, geteilte `@include`-Fragmente in `_includes/`, ausführbare Begleit-
dateien als install-materialisierte Assets.

```
content/skills/lmd-brainstorm/
  SKILL.md                         # dünner Discovery-Stub (ersetzt bestehenden Stub)
  body.lmd.md                      # 8 @phase-Blöcke (ersetzt 3-Phasen-Skelett)
  _includes/
    brainstorm-gate.lmd.md         # NEU: Trip-Wire (HARD-GATE + "Too Simple"-Anti-Pattern), @include je Disziplin-Phase
  companions/
    spec-reviewer.lmd.md           # aus spec-document-reviewer-prompt.md
    visual-companion.lmd.md        # aus visual-companion.md (Prosa, JIT)
  scripts/                         # Assets (install-materialisiert, Muster render-graphs.js)
    server.cjs
    helper.js
    frame-template.html
    start-server.sh
    stop-server.sh
```

Verdrahtung in `src/skills.rs`: `include_str!`-Konstanten + Einträge in `SKILLS` und
`COMPANIONS`. Das Fragment wird per `@include brainstorm-gate` (blanker Name) aus
`_includes/` aufgelöst — wie `skill-authoring-core` / `test-first-core` bei den
Schwester-Skills. COVERAGE-Zeilen in `src/availability.rs`. Asset-Materialisierung in
`src/skill_install.rs`.

### Phasen (`body.lmd.md`)

Jede Phase wird über `capture_phase_bodies` isoliert (kein Cross-Phase-Leak). Die
Disziplin-Phasen ziehen das geteilte Gate-Fragment per `@include`.

| Phase | Inhalt (Original-Quelle: `brainstorming/SKILL.md`) | @include / Direktiven |
|---|---|---|
| `pre-context` | HARD-GATE + „This Is Too Simple To Need A Design"-Anti-Pattern + 9-Punkt-Checklist + „Process Flow"-`dot` (verbatim) + „Key Principles" | `@include brainstorm-gate` |
| `explore` | „Explore project context" (Dateien/Docs/Commits), Scope-Assessment, Dekomposition großer Projekte in Sub-Projekte | `@include brainstorm-gate` + `@read @search @list` |
| `questions` | „Ask clarifying questions" — eine Frage/Nachricht, Multiple-Choice bevorzugt, YAGNI; **JIT-Visual-Companion-Offer-Trigger** (eigene Nachricht) | `@include brainstorm-gate` |
| `approaches` | „Propose 2-3 approaches" — Trade-offs, Empfehlung zuerst | `@include brainstorm-gate` + `@graph @impact` |
| `present-design` | „Present design" abschnittsweise + Freigabe; „Design for isolation and clarity"; „Working in existing codebases"; pro Frage Browser-vs-Terminal-Entscheidung | `@include brainstorm-gate` |
| `write-spec` | „Write design doc" → `docs/lean-md/specs/YYYY-MM-DD-<topic>-design.md`, dann commit | `@edit @remember` |
| `self-review` | „Spec Self-Review" (Placeholder/Konsistenz/Scope/Ambiguität) → **`@dispatch … companion="spec-reviewer" role=review`**; „User Review Gate" | `@dispatch @review` |
| `handoff` | Approved Spec → Übergabe an `lmd-writing-plans` (terminaler State) | `@dispatch @handoff` |

Render: `ctx_md_render(skill="lmd-brainstorm", phase="<name>")`.

**Layout-Konvention (der −95 %-Hebel):** der schwere `@include`-Gate-Block steht in
jeder Disziplin-Phase als Trip-Wire, aber jede Phase wird **isoliert** gerendert —
der Agent zieht nur den Block, an dem er gerade arbeitet.

### Geteiltes Gate-Fragment `brainstorm-gate`

Trip-Wire, in **jede Disziplin-Phase** (`pre-context`, `explore`, `questions`,
`approaches`, `present-design`) `@include`d. Trägt:

- den **HARD-GATE** (verbatim): „Do NOT invoke any implementation skill, write any
  code, scaffold any project, or take any implementation action until you have
  presented a design and the user has approved it."
- die Kernzeile des „Too Simple"-Anti-Patterns: jedes Projekt durchläuft den Prozess;
  „simple" Projekte sind gerade dort, wo ungeprüfte Annahmen Arbeit verschwenden.

**Abgrenzung:** eigenständiges Fragment, **nicht** `skill-authoring-core` /
`test-first-core` der Schwester-Skills. Das Gate ist die brainstorm-eigene Disziplin
(kein Code vor Freigabe), parallel zum Iron Law der TDD-Skills — Inhalt, kein
File-`@include` voneinander.

### Companions (2)

Render: `ctx_md_render(skill="lmd-brainstorm", companion="<name>")`. `phase` und
`companion` sind mutually exclusive.

| Companion | Quelle im Original | Besonderheit |
|---|---|---|
| `spec-reviewer` | `spec-document-reviewer-prompt.md` (Voll-Port) | Check-Tabelle (Completeness/Consistency/Clarity/Scope/YAGNI) + Calibration + Output-Format. **Via `@dispatch` als Brief dispatchbar.** |
| `visual-companion` | `visual-companion.md` (Voll-Port) | When-to-use (Browser-vs-Terminal-Test), „The Loop", Content-Fragmente-vs-Full-Doc, CSS-Klassen, Events-Format, Design-Tips, **Pro-Plattform-Server-Start-Matrix (verbatim)**. JIT geladen. |

#### `spec-reviewer` — Dispatch (kein Engine-Change)

`self-review` materialisiert in der Phase einen Dispatch-Block:

```
@dispatch skill="lmd-brainstorm" companion="spec-reviewer" role=review to_agent="{{ controller_id }}"
```

`@dispatch` komponiert daraus contract (auto-prepended) + Brief (Reviewer-Companion,
work-lazy gerendert) + Bootstrap; `role=review` ist bereits valide. Der Subagent prüft
`SPEC_FILE_PATH`, posted `ctx_agent action=post category=finding`, gibt Status
(`Approved | Issues Found`) an den Controller zurück. **`@dispatch` spawnt nicht** —
es komponiert den Prompt; den Spawn macht der Controller/die Harness.

#### `visual-companion` — JIT + Fidelity-Punkt Harness-Matrix

- **Just-in-time:** nur wenn in `questions`/`present-design` eine echte visuelle
  Frage auftaucht; das Offer ist eine **eigene Nachricht** (Original-Regel). Geladen
  via `ctx_md_render(skill="lmd-brainstorm", companion="visual-companion")`.
- **Fidelity-Punkt (vom Nutzer bestätigt):** Die **Pro-Plattform-Server-Start-Matrix**
  (Claude Code / Codex / Gemini CLI / Copilot CLI / „Other environments") wird
  **verbatim** mitportiert, damit `lmd-brainstorm` unter fremden Harnesses lauffähig
  bleibt (Backgrounding eines Node-Servers über Conversation-Turns ist
  harness-spezifisch).
- **Reference-Closure-Edits:** Datei-Schreiben via Write/`ctx_edit`, **nie**
  cat/heredoc; `.superpowers/brainstorm/`-Pfade → lean-md-eigener Pfad (z. B.
  `.lean-ctx/lean-md/brainstorm/`); `scripts/`-Referenzen zeigen auf den
  install-materialisierten Skill-Dir.

### Assets: `scripts/` (5 Dateien)

`.cjs/.js/.html/.sh` sind Text → `include_str!`-embedded. `skill install`
materialisiert sie nach `.claude/skills/lmd-brainstorm/scripts/` (neuer/erweiterter
Asset-Schritt in `skill_install.rs`, idempotent/absent-only nach Vorbild
`render-graphs.js`); die `.sh` werden `chmod +x` gesetzt. Laufzeit-Deps (node) liegen
beim Nutzer — wie im Original. Kein Rendering durch `ctx_md_render`.

| Asset | Quelle |
|---|---|
| `server.cjs` | `brainstorming/scripts/server.cjs` |
| `helper.js` | `brainstorming/scripts/helper.js` |
| `frame-template.html` | `brainstorming/scripts/frame-template.html` |
| `start-server.sh` | `brainstorming/scripts/start-server.sh` |
| `stop-server.sh` | `brainstorming/scripts/stop-server.sh` |

**R2-Gate:** jede `.superpowers/`-Referenz in den Scripts auf den lean-md-Pfad ziehen;
Grep-Gate gegen `superpowers`-Reststellen über Seeds **und** Assets.

### SKILL.md-Stub

- Frontmatter: `name: lmd-brainstorm` / `description:` 1:1 aus dem Original
  („You MUST use this before any creative work …").
- Body: Overview + HARD-GATE-Kurzhinweis + Phasen-Pointer (8) + Companion-Pointer (2)
  + Hinweis „nie von Disk lesen — immer via `ctx_md_render`".
- Das Process-Flow-Flowchart steht **nicht** inline, sondern in der `pre-context`-Phase
  (verbatim ```dot).

## Reference-Closure (superpowers-Unabhängigkeit)

| Original-Verweis | lmd-Ziel |
|---|---|
| `writing-plans` (terminaler State, „invoke the writing-plans skill") | **`lmd-writing-plans`** (Name-Pointer; Port = nächste Folge-Arbeit, s. Non-Goals) |
| „frontend-design, mcp-builder, any other implementation skill" (HARD-GATE-Negativliste) | generisch umformuliert („any implementation skill"), kein superpowers-Skillname |
| `spec-document-reviewer-prompt.md` | Companion `spec-reviewer` |
| `visual-companion.md` | Companion `visual-companion` |
| `scripts/*` | Assets (install-materialisiert) |
| `.superpowers/brainstorm/` (Server-Pfad) | lean-md-Pfad `.lean-ctx/lean-md/brainstorm/` |
| `references/*-tools.md` (Tool-Mapping der using-superpowers-Meta-Skill) | entfällt — lean-md spricht direkt lean-ctx-Tools (`ctx_read`/`ctx_edit`/…); Harness-Agnostik via `ctx_md_render` |

## Fidelity-Coverage-Matrix (Audit-Artefakt)

Vollständige Quell-Inventur — jede Original-Sektion/Datei → genau ein lmd-Ziel:

| Original-Element (`brainstorming/`) | lmd-Ziel |
|---|---|
| `SKILL.md` Frontmatter + Overview | Stub |
| HARD-GATE-Block | `_includes/brainstorm-gate` + Stub-Kurzhinweis |
| „Anti-Pattern: This Is Too Simple To Need A Design" | `_includes/brainstorm-gate` (Kernzeile) + `pre-context` (voll) |
| „Checklist" (9 Items) | Phase `pre-context` |
| „Process Flow" (```dot) | Phase `pre-context` (verbatim) |
| „The Process": Understanding the idea | Phase `explore` + `questions` |
| „The Process": Exploring approaches | Phase `approaches` |
| „The Process": Presenting the design | Phase `present-design` |
| „Design for isolation and clarity" | Phase `present-design` |
| „Working in existing codebases" | Phase `present-design` |
| „After the Design": Documentation | Phase `write-spec` |
| „After the Design": Spec Self-Review | Phase `self-review` |
| „After the Design": User Review Gate | Phase `self-review` |
| „After the Design": Implementation (→ writing-plans) | Phase `handoff` |
| „Key Principles" | Phase `pre-context` |
| „Visual Companion" (Offer-Mechanik, JIT) | Phase `questions` (Trigger) + Companion `visual-companion` |
| `spec-document-reviewer-prompt.md` | Companion `spec-reviewer` |
| `visual-companion.md` (inkl. Harness-Matrix) | Companion `visual-companion` |
| `scripts/{server.cjs, helper.js, frame-template.html, start-server.sh, stop-server.sh}` | Assets |

## Validierung (TDD des Plans)

### Rust / nextest (`cargo nextest run`, nie `cargo test`)

- `skill_registered` — `all_skill_bodies`/`skill_body` enthalten `lmd-brainstorm`.
- `fragment_consistency` — built-in `brainstorm-gate` == On-Disk-Seed (byte-stabil).
- `phase_isolation` — keine der 8 Phasen leakt Inhalt einer anderen.
- `companion_render` — beide Companions lösen nicht-leer auf; `phase`+`companion`
  mutually exclusive.
- `gate_trip_wire` — Disziplin-Phasen tragen den HARD-GATE-Marker (via `@include`);
  `write-spec`/`self-review`/`handoff` (Nicht-Gate-Phasen) sind sauber abgegrenzt.
- `dispatch_spec_reviewer_composes` — `@dispatch … companion="spec-reviewer"
  role=review` rendert contract + Reviewer-Brief (Check-Tabelle) + Bootstrap,
  `role=review`. (Nutzt vorhandene Bridge — kein Engine-Change.)
- `cli_eq_mcp` — `render_skill`/`render_companion` byte-identisch über CLI + MCP.
- `coverage_rows` — `availability.rs` trägt `lmd-brainstorm`-Zeilen (Phase → Direktive
  → lean-ctx-Backing) inkl. Companion- + Dispatch-Zeile.
- `asset_materialization` — `install_skill` schreibt alle 5 `scripts/`-Dateien,
  idempotent (absent-only), `.sh` ausführbar, korrektes Ziel.
- `reference_closure_grep` — kein `superpowers`-String in Seeds **oder** Assets.

### Subagent-Pressure-Test (Iron Law der Skill selbst — Auftrag „lmd-writing-skills nutzen")

- **RED-Baseline:** Subagent **ohne** die Skill bekommt eine Brainstorm-Aufgabe →
  springt zu Implementierung / überspringt Design-Freigabe → Rationalisierungen
  verbatim dokumentieren.
- **GREEN:** gleicher Lauf **mit** gerendertem `lmd-brainstorm` → Agent compliant
  (HARD-GATE respektiert, eine Frage/Nachricht, Design vor Freigabe, Spec-Self-Review).

### Fidelity-Audit (kein Verlust prüfbar)

- Rust-Test: jede Phase + jeder Companion + jedes Asset rendert/materialisiert
  nicht-leer.
- Manueller Section-by-Section-Abgleich Original ↔ Port anhand der Coverage-Matrix.

## Non-Goals

- **Kein Visual-Companion-Server-Rewrite.** Scripts werden 1:1 mitgeliefert; node
  liegt beim Nutzer.
- **Kein neuer CLI-Subcommand.** `lean-md skill install` existiert bereits; nur der
  Asset-Schritt für `lmd-brainstorm/scripts/` kommt hinzu.
- **Kein Engine-/Render-Core-Change** (`rushdown`/`evalexpr`, Bridges).
- **Skill-Discovery-Targets für fremde Harnesses (R4).** v1 materialisiert den Stub
  nach `.claude/skills/` (honoriert `CLAUDE_CONFIG_DIR`); `--agent <x>`-Targets für
  Codex/Gemini/Copilot = Folge-Arbeit. Betrifft **nicht** die Nutzbarkeit über
  Kanal ① (`ctx_md_render` ist harness-agnostisch).
- **`lmd-writing-plans`-Port** = nächste Folge-Arbeit (eigenes Spec). Die
  `handoff`-Phase verweist bereits als Name-Pointer darauf.

## Risiken

- **R1 — Phase-Isolation mit Gate-`@include`:** das in 5 Phasen ge-`@include`te
  `brainstorm-gate` darf den Isolations-Test nicht brechen (der Block ist erwartet
  in mehreren Phasen — Test prüft Cross-Phase-Leak **anderer** Inhalte, nicht des
  geteilten Gates). Test-Design entsprechend (wie `skill-authoring-core` bei
  writing-skills).
- **R2 — `scripts/`-Pfad-Adaption:** `.superpowers/`-Referenzen müssen restlos auf
  den lean-md-Pfad gezogen werden (Grep-Gate über Seeds + Assets).
- **R3 — Harness-Matrix-Fidelity:** die Pro-Plattform-Server-Start-Tabelle muss
  verbatim erhalten bleiben; offen-lassen, welche Harness genutzt wird (kein
  Claude-Hardcode in der Prosa).
- **R4 — Discovery-Target fremder Harnesses:** s. Non-Goals (Folge-Arbeit, kein
  v1-Blocker).

## Globale Constraints

- Tests: immer `cargo nextest run`, nie `cargo test`. Crate standalone, Repo-Root
  (kein `cd`, kein `--manifest-path`).
- Shell: kein `&&`/`||`/`;`-Chaining — jede Invocation einzeln.
- Vor jedem `git add` (je geänderte Code-Datei): `cargo fmt`.
- No worktrees — direkt auf `feat-lmd-v2`.
- Determinismus #498: keine Timestamps/Counter/Random; embedded == on-disk;
  `CliBackend` == `McpBackend`.
- Code/Kommentare Englisch; Interaktion/Commits Deutsch mit Umlauten.
- Naming: ausgeschrieben `lmd-brainstorm`.

## Plan

TDD-strukturiert, Deutsch, `docs/lean-md/plans/2026-06-30-lmd-brainstorm-port.md`,
Checkbox-Tasks im Format des Schwester-Plans
(`2026-06-29-lmd-writing-skills-port.md`). Grobe Task-Sequenz:

1. SKILL.md-Stub + Registry-Eintrag (`skill_registered` RED→GREEN).
2. `brainstorm-gate`-Fragment in `_includes/` + Fragment-Consistency-Gate.
3. `body.lmd.md` 8 Phasen + Phasen-Isolations-Tests + `gate_trip_wire`.
4. Companion `spec-reviewer` + Dispatch-Compose-Test (vorhandene Bridge).
5. Companion `visual-companion` (inkl. Harness-Matrix verbatim) + Render-Test + CLI==MCP.
6. Assets `scripts/` (5) + `skill_install`-Asset-Schritt + `asset_materialization` + R2-Grep-Gate.
7. COVERAGE-Zeilen in `availability.rs` + `availability-audit.md`.
8. Fidelity-Audit (Rust-Coverage + manueller Abgleich) + `reference_closure_grep`.
9. Subagent-Pressure-Test (RED-Baseline → GREEN).
10. Full-Gate: `cargo fmt`, `cargo nextest run`, clippy `-D warnings`, Determinismus, Render-Smoke.
