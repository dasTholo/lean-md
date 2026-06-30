# lmd-brainstorm — Native lean-md Port der Brainstorming-Skill (Design-Spec)

Status: **SUPERSEDED** (2026-06-30) → ersetzt durch
`docs/lean-md/specs/2026-06-30-lmd-brainstorm-port-design.md`. Grund: Seit diesem
Entwurf haben `lmd-test-driven-development` + `lmd-writing-skills` die realisierten
Konventionen etabliert (`companions/`-Ordner inkl. `group/name`, `_includes/` für
`@include`-Fragmente, Asset-Materialisierung via `skill install`, `@dispatch` mit
`companion=`-Brief + Rolle `review`/`test` und auto-prepended `dispatch-contract`).
Das Nachfolge-Spec biegt den Port auf diese Konventionen um (kein Engine-Change nötig).

Datum: 2026-06-29 · Branch: `feat-lmd-v2`

**Referenz-Specs (Abgleich §9):**

- `2026-06-26-lean-md-standalone-addon-design.md` — **aktuelle autoritative Baseline**
  (supersedet die v2-native; verifizierter IST: 30 Bridges, `skills.rs`/`seeds.rs`/`availability.rs`,
  zero-config, Decoupling vollzogen).
- `2026-06-22-lmd-lean-ctx-native-design-v2.md` — **superseded**, trägt aber die Detailbeschreibung
  der **Runtime-Materialisierung** (Schicht A/B). Übernommen **mit** einer durch das Decoupling
  erzwungenen Korrektur: der dort beschriebene lean-ctx-seitige Skill-Installer
  (`install_lmd_skills`, `[lean-md]`-Config) wurde **entfernt** → der Install zieht in lean-md um (D5/§4.4).

## 1. Ziel & Kontext

Die superpowers-`brainstorming`-Skill wird als **nativer lean-md-Skill** `lmd-brainstorm`
portiert. „Nativ" heißt: der Skill-Body ist **binary-embedded** und wird **phasenweise**
über `ctx_md_render(skill="lmd-brainstorm", phase=<phase>)` gerendert — der −88…−95 %-Token-Hebel
(v2-Spec §5.4/§5.5). Alle Brainstorming-Workflow-Schritte fließen über **kuratierte lean-md-Direktiven**
(`@read`/`@search`/`@graph`/…), deren lean-ctx-Backing im prüfbaren `availability-audit` festliegt.

Zwei Companion-Dateien der Original-Skill werden mitportiert:

- `spec-document-reviewer-prompt.md` → `@dispatch`-Contract-Fragment
- `visual-companion.md` (+ `scripts/`) → Prosa-Seed (render-on-invoke) + embedded Node-Server

**Nicht-Ziel:** kein Server-Rewrite der Visual-Companion-`scripts/`; keine neue zweite
Materialisierungs-Konvention; kein Eingriff in den render core (`rushdown`/`evalexpr`).

### 1.1 Ausgangszustand (bereits im Repo)

- `content/skills/lmd-brainstorm/{SKILL.md, body.lmd.md}` — Stub + 3-Phasen-Skelett mit Platzhalter-Markern.
- `content/gloss/directives.lmd.md` — vollständiger Direktiven-Katalog (~25).
- `content/tooling/availability-audit.md` — kuratierte Coverage-Matrix + bewusste Gap-Liste.
- `content/core/{hard-rules,dispatch-contract}.lmd.md`, `core/_fragments/tool-quick-ref.lmd.md`.
- `src/skills.rs` — `skill_body()` + `render_skill()` mit Phase-Isolation (`capture_phase_bodies`).
- `src/fragments.rs` — `include_str!`-Embedding + Fragment-Consistency-Gate.

## 2. Entscheidungen (alle Forks geschlossen)

| #   | Fork                  | Entscheidung                                                                                                                                                                                                                                      |
|-----|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| D1  | Tool-Coverage         | **Kuratiert pro Phase**; `availability-audit.md` = Single Source; Gap-Liste bleibt. „Alle Tools" = lückenlose **Phasen**-Abdeckung, nicht jede Direktive überall.                                                                                 |
| D2  | Body-Organisation     | **A+C**: Monolith-`body.lmd.md` mit 8 `@phase`-Blöcken; Fragment-Extraktion (`@include`) nur bei echtem Reuse.                                                                                                                                    |
| D3  | Spec-Reviewer         | **`@dispatch`-Contract-Fragment** (core-first + Delta, D-7-Kanon).                                                                                                                                                                                |
| D4  | Visual-Companion      | **Prosa-Seed** (render-on-invoke, out-of-band Target) **+ `scripts/` 1:1** aus superpowers, an lean-md adaptiert.                                                                                                                                 |
| D5  | scripts/-Distribution | **`include_str!`-embedded**, vom Install-Kommando rausgeschrieben (wie lean-ctx `install_claude_skill`). Single-Binary.                                                                                                                           |
| D6  | Materialisierung      | Zwei Schichten gem. v2-native: **A** global `~/.claude/skills/`, **B** Projekt-Overlay `.lean-ctx/lean-md/`.                                                                                                                                      |
| D7  | Body-Override         | **Erweiterung des v2-native:** Override-Auflösung wird auf den **Skill-Body** ausgedehnt → lokale Phasen-Iteration ohne Recompile.                                                                                                                |
| D8  | Overlay-Pfad          | **`.lean-ctx/lean-md/`** als **hartkodierte Konvention** (lean-md ist zero-config, §2.1; **kein** `[lean-md] contracts_dir`-Key mehr — der wurde beim Decoupling aus lean-ctx entfernt). Optionaler Env-Override zulässig, kein Config-Discovery. |
| D10 | Opt-in-Gate           | Zero-config: Opt-in = **`lean-md skill install` ausführen** (Invocation IST der Consent). **Kein** `[lean-md] skill`-Config-Flag (gibt es nicht mehr).                                                                                            |
| D9  | Lokaler Test          | In-Repo: `CLAUDE_CONFIG_DIR=<repo>/.claude` → `skill install` schreibt ins **repo-lokale `.claude/`**; cargo-Tests via `tempfile::tempdir()`. Kein globaler Eingriff.                                                                             |

## 3. Architektur

### 3.1 Zwei Install-Kanäle (Spiegel von lean-ctx)

| Kanal                 | Inhalt                                         | Mechanismus                                                                           |
|-----------------------|------------------------------------------------|---------------------------------------------------------------------------------------|
| **① Runtime/Tool**    | Render-Bodies → `ctx_md_render`/`ctx_md_check` | Addon-Gateway-Server (`lean-ctx addon add lean-md`) — **bereits vorhanden**           |
| **② Skill/Discovery** | `SKILL.md`-Stub + `scripts/` → Skills-Dir      | **neu:** opt-in `lean-md skill install` schreibt `include_str!`-embedded Dateien raus |

**Agent-Agnostik (wichtig):** Der **Skill-Inhalt ist nicht Claude-spezifisch**. Über Kanal ① ist er für
**jeden** MCP-fähigen Agenten (Codex/GPT, Gemini, Pi, Cursor, Copilot, …) erreichbar, der das lean-md-Addon
über das lean-ctx-Gateway anspricht — alle rufen dasselbe `ctx_md_render(skill, phase)` und bekommen
denselben gerenderten Prompt. Kanal ② (`SKILL.md`-Stub) ist **nur** die **Auto-Discovery-Bequemlichkeit**:
er bewirkt, dass eine Harness den Skill von selbst als `/lmd-brainstorm` anzeigt. Ohne Stub bleibt der Skill
nutzbar — der Agent ruft das MCP-Tool direkt. Das **Materialisierungs-Ziel** des Stubs ist harness-spezifisch
(`~/.claude/skills/` bei Claude Code; andere Harnesses haben eigene/keine Konventionen, vgl. lean-ctx `init --agent <x>`).

Der `addon add`-Pfad fasst Skills **nicht** an (Contract-Schritte 1–7 = nur Gateway-Server + `installed.json`).
Kanal ② ist davon entkoppelt und **opt-in durch Invocation**: erst `lean-md skill install` materialisiert
den Stub. **Kein** Config-Flag-Gate — lean-ctx' `[lean-md]`-Block + `install_lmd_skills` wurden beim
Decoupling entfernt (Baseline §2.2), daher trägt **lean-md** den Install selbst (§4.4).

### 3.2 Seed-Layout (alles unter `content/`, alles embedded)

```
content/skills/lmd-brainstorm/
  SKILL.md                 ~ Stub (Frontmatter + render-on-invoke-Hinweis); embedded → von skill install rausgeschrieben
  body.lmd.md              ~ 3 → 8 @phase-Blöcke; embedded → via ctx_md_render serviert
  spec-reviewer.lmd.md     + @dispatch-Fragment (Delta-only); embedded → ctx_md_render-Target
  visual-companion.lmd.md  + Prosa-Seed; embedded → ctx_md_render-Target (out-of-band)
  scripts/                 + aus superpowers kopiert, an lean-md adaptiert; embedded → von skill install rausgeschrieben
    server.cjs, helper.js, frame-template.html, start-server.sh, stop-server.sh
content/core/_fragments/
  hard-gate.lmd.md         + C-Extraktion: HARD-GATE-Block (mehrfach @include)
```

### 3.3 Runtime-Materialisierung (erweitert v2-native §Runtime-Materialisierung, Phase 10)

| Seed                                                      | Laufzeit-Ziel                                    | Scope                                                  |
|-----------------------------------------------------------|--------------------------------------------------|--------------------------------------------------------|
| `core/`, `core/_fragments/`                               | bleibt im Binary                                 | embedded/locked                                        |
| `skills/<n>/SKILL.md`, `skills/<n>/scripts/*`             | `<CLAUDE_CONFIG_DIR                              | ~/.claude>/skills/<n>/` **global** (Discovery), opt-in |
| `skills/<n>/{body,spec-reviewer,visual-companion}.lmd.md` | bleibt im Binary **— oder Projekt-Overlay (D7)** | embedded, **override-fähig**                           |

**Auflösungs-Reihenfolge (D7-Erweiterung):** `render_skill()` prüft **zuerst**
`<contracts_dir>/skills/<n>/<seed>.lmd.md` im aktuellen Projekt-Overlay; existiert die Datei →
sie wird gerendert (Phase-Isolation läuft auf der Overlay-Quelle), sonst der `include_str!`-Const.
Damit lässt sich an einer **Phase lokal iterieren, ohne Recompile**.

## 4. Komponenten

### 4.1 Body — die 8 Phasen (kuratierte Direktiven aus `availability-audit.md`)

`@phase "name" … @phase-end`; `ctx_md_render(phase=X)` rendert **nur** den Block X.

| Phase            | Inhalt (Kern)                                                                                   | Direktiven                  |
|------------------|-------------------------------------------------------------------------------------------------|-----------------------------|
| `pre-context`    | `@include hard-rules` + `@include dispatch-contract` + `@include hard-gate` + 8-Punkt-Checklist | — (Vertrag)                 |
| `explore`        | Projektkontext (Dateien/Docs/Commits), Scope-Assessment, Dekomposition                          | `@read @list @search @find` |
| `questions`      | Eine Frage/Nachricht, Multiple-Choice bevorzugt, YAGNI; **JIT Visual-Companion-Offer-Trigger**  | — (Dialog)                  |
| `approaches`     | 2–3 Ansätze, Trade-offs, Empfehlung zuerst                                                      | `@graph @impact`            |
| `present-design` | Abschnittsweise + Freigabe; Isolation/Klarheit; pro Frage Browser-vs-Terminal                   | — (Dialog)                  |
| `write-spec`     | Spec → `docs/lean-md/specs/YYYY-MM-DD-<topic>-design.md`, dann commit                           | `@edit @remember`           |
| `self-review`    | Platzhalter/Konsistenz/Scope/Ambiguität; **dann `@dispatch spec-reviewer`**                     | `@review` (+`@dispatch`)    |
| `handoff`        | Approved Spec an Controller für `writing-plans`                                                 | `@dispatch @handoff`        |

**Layout-Konvention (v2-native §5.4, „der −95 %-Hebel"):** der schwere `@include`-Block **nur** in
`pre-context`; alle Folge-Phasen lean. Strukturell garantierter Token-Gewinn.

### 4.2 Spec-Reviewer (`spec-reviewer.lmd.md`)

`@dispatch`-Fragment, **core-first komponiert**: `hard-rules` + `dispatch-contract` (embedded)
**+ Reviewer-Delta**:

- Check-Tabelle: Completeness · Consistency · Clarity · Scope · YAGNI
- Calibration: „nur Issues flaggen, die echte Planungsprobleme verursachen"
- Output: `Status (Approved|Issues Found)` · `Issues` · `Recommendations (advisory)`

`self-review` ruft `@dispatch spec-reviewer` → Subagent prüft `SPEC_FILE_PATH`, posted
`ctx_agent action=post category=finding`, gibt Status an Controller zurück.
**Delta-only — keine Vertrags-Duplikate** (D-7).

### 4.3 Visual-Companion (`visual-companion.lmd.md` + `scripts/`)

- **Prosa-Seed** (out-of-band Render-Target, nicht in der 8-Phasen-Sequenz): Browser-vs-Terminal-Test,
  der Loop, Content-Fragmente-vs-Full-Doc, CSS-Klassen, Events-Format. Tool-Refs auf lean-ctx
  umgeschrieben: Datei-Schreiben via Write/`ctx_edit`, **nie** cat/heredoc.
- **`scripts/`**: 1:1 aus superpowers, embedded. `start-server.sh --project-dir` zeigt auf den
  materialisierten Skill-Dir; `.superpowers/brainstorm/` → lean-md-eigener Pfad (z. B. `.lean-ctx/lean-md/brainstorm/`).
- **Just-in-time geladen:** nur wenn in `questions`/`present-design` eine echte visuelle Frage auftaucht,
  via `ctx_md_render(skill="lmd-brainstorm", phase="visual-companion")`.

### 4.4 `lean-md skill install` / `skill remove` (`src/skill_install.rs`, neu)

**Warum in lean-md (nicht lean-ctx):** der lean-ctx-seitige `install_lmd_skills`/`[lean-md]`-Pfad
des v2-native wurde beim Decoupling **entfernt** (Baseline §2.2). Da der Endnutzer **nur das Binary**
hat, müssen alle auf Platte zu schreibenden Dateien `include_str!`-embedded sein → der Install **muss**
in lean-md leben. Spiegelt lean-ctx `install_claude_skill`/`remove_claude_skill`:

- `include_str!` für `SKILL.md` + `scripts/*` → schreibt nach `claude_state_dir()`-`/skills/lmd-brainstorm/`
  (honoriert `CLAUDE_CONFIG_DIR`, v2/#596), `chmod +x` der `.sh`. Atomic, idempotent.
- `skill remove` entfernt **nur** das lmd-eigene Dir.
- Opt-in = **Invocation** (zero-config, kein Flag); CLI-Subcommand-Wiring in `src/args.rs`
  (neben `render`/`check`/`mcp`).

## 5. Wiring & Gates

**Wiring:**

- `src/skills.rs`: zusätzliche Render-Targets (`spec-reviewer`, `visual-companion`) — neue
  `include_str!`-Consts + Match-Arms in `skill_body()`; Body-Override-Auflösung (D7) in `render_skill()`.
- `src/fragments.rs`: `hard-gate`-Fragment + `@include`-Auflösung; Fragment-Consistency-Gate auf neue Seeds erweitern.
- `content/tooling/availability-audit.md`: Zeilen für `visual-companion` (JIT) ergänzen; Gap-Liste prüfen.

**Gates (`cargo nextest run`, nie `cargo test`):**

1. Phase-Isolation für **alle 8** Phasen (kein Cross-Phase-Leak) — bestehenden Test erweitern.
2. Dispatch-Compose: `spec-reviewer` = `hard-rules` + `dispatch-contract` + Delta.
3. Fragment-Consistency-Gate grün (built-in == on-disk für **alle** neuen Seeds).
4. availability-Coverage-Gate: `availability.rs::COVERAGE` ↔ `availability-audit.md`.
5. `skill_install`-Roundtrip (tempdir + `CLAUDE_CONFIG_DIR`-Pin): install→Dateien da, remove→weg.
6. Body-Override (D7): Overlay-Datei vorhanden → Overlay gerendert; absent → embedded.
7. Determinismus (#498): keine Timestamps/Counter, byte-stabil; `CliBackend`==`McpBackend`.

## 6. Lokaler Test-Flow (in-Repo, kein globaler Eingriff)

1. Seeds + `skill_install` + Wiring schreiben.
2. `cargo nextest run --manifest-path Cargo.toml` → alle Gates grün.
3. `cargo fmt` vor jedem `git add` (Standalone-Crate, `Cargo.toml`+`src/` im Repo-Root).
4. Manueller E2E: `CLAUDE_CONFIG_DIR=<repo>/.claude lean-md skill install` → repo-lokales
   `.claude/skills/lmd-brainstorm/`; neue Session → Harness entdeckt `/lmd-brainstorm` → Phasen rendern.
5. `.lean-ctx/lean-md/` + repo-lokales `.claude/` zur `.gitignore` hinzufügen.

## 7. Risiken & offene Punkte

- **R1 — Body-Override-Sicherheit:** Overlay-Pfad ist PathJail-gebunden (Spec §6); keine Eskalation außerhalb
  `contracts_dir`.
- **R2 — `scripts/`-Pfad-Adaption:** Original referenziert `.superpowers/`; jede Referenz muss auf den lean-md-Pfad
  gezogen werden (Test-/Grep-Gate gegen `superpowers`-Reststellen).
- **R3 — Out-of-band-Target:** `visual-companion` ist keine 8er-Phase; `render_skill` muss ein Nicht-Sequenz-Target
  auflösen, ohne die Phase-Sequenz zu verletzen.
- **R4 — Auto-Discovery-Ziel für nicht-Claude-Harnesses (nur Komfort, keine Nutzbarkeit):** Der Skill ist über
  Kanal ① für **jeden** MCP-Agenten nutzbar (§3.1 Agent-Agnostik). Offen ist nur, wohin `skill install` den
  `SKILL.md`-Stub für **andere** Harnesses schreibt (Codex/Gemini/Copilot-Konventionen). v1 zielt auf Claude Code
  (`~/.claude/skills/`); `--agent <x>`-Targets = Folge-Arbeit. Betrifft **nicht**, ob der Skill anderswo funktioniert.
- **F1 — `lean-md skill install` als eigener Subcommand — GELÖST durch Baseline-Abgleich:** der v2-native nannte einen
  lean-ctx-getriggerten Pfad (`install_lmd_skills` + `[lean-md]`-Config). Dieser wurde beim **Decoupling aus lean-ctx
  entfernt** (Baseline §2.2). Da der Endnutzer nur das Binary hat, **muss** lean-md den Install selbst tragen → eigener
  Subcommand. Keine offene Frage mehr.

## 8. Scope-Abgrenzung

- **In v1:** 8-Phasen-Body, spec-reviewer-Fragment, visual-companion-Seed+scripts, `skill install/remove`,
  Body-Override (D7), Wiring, alle Gates.
- **Folge-Spec:** weitere lmd-Skills über `lmd-brainstorm` hinaus; Skill-Discovery für weitere Agenten (R4);
  Visual-Companion-Server-Rewrite (verworfen, YAGNI).

## 9. Abgleich mit Referenz-Specs (Vollständigkeits-Check)

Geprüft gegen `2026-06-26-standalone-addon` (Baseline) + `2026-06-22-native-v2` (Materialisierungs-Detail):

| Aspekt der Referenz-Specs                                                                                   | In diesem Spec berücksichtigt?                                                      |
|-------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| `ctx_md_render(skill, phase)` render-on-invoke (native-v2 §5.5, baseline §5.2.1)                            | ✅ §1, §4.1 — phasenweiser Render, −95 %-Hebel                                       |
| `@dispatch`/`@handoff` **bereits implementiert** (baseline §2.1 `bridges/{dispatch,handoff,addressing}.rs`) | ✅ §4.1 self-review/handoff, §4.2 — keine Neuimplementierung, nur Nutzung            |
| `skills.rs`/`seeds.rs`/`availability.rs` existieren (baseline §2.1)                                         | ✅ §5 Wiring — Erweiterung dieser Module, nicht Neubau                               |
| **zero-config**, keine eigene TOML/JSON (baseline §3.4c/§5.4)                                               | ✅ D8/D10 korrigiert — Overlay = Konvention, Opt-in = Invocation                     |
| lean-ctx-Skill-Installer **entfernt** (baseline §2.2)                                                       | ✅ F1 gelöst — Install zieht in lean-md (§4.4)                                       |
| Runtime-Materialisierung Schicht A/B (native-v2 §Runtime-Materialisierung)                                  | ✅ §3.3 + D7-Erweiterung (Body-Override)                                             |
| Determinismus #498, byte-stabile Seeds, Fragment-Consistency-Gate (baseline §3.3)                           | ✅ §5 Gate 3+7                                                                       |
| availability-Coverage als prüfbares Gate (native-v2 §5.4, baseline)                                         | ✅ §5 Gate 4, D1                                                                     |
| Gateway-Namespace `lean-md::ctx_md_render` (baseline §5.1.1)                                                | ✅ kein Konflikt — Skill ruft `ctx_md_render`; Stub nutzt den real exponierten Namen |
| **Kein** Agent-Spawn durch `@dispatch` (baseline §1 „bewusst NICHT")                                        | ✅ §4.2 — `@dispatch` komponiert Prompt, Spawn macht der Controller/Harness          |
| CLI-Surface `render`/`check`/`mcp` (baseline §5.2.1)                                                        | ✅ §4.4 — `skill install` reiht sich ein                                             |

**Kein vergessener Aspekt identifiziert.** Einzige bewusste Erweiterung über die Baseline hinaus:
Body-Override (D7) — als solche markiert.
