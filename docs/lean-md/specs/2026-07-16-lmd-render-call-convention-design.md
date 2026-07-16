# lean-md — Render-Aufrufkonvention, `.ext`-Komposition, Tool-Alias

**Status:** approved (Brainstorm 2026-07-16)
**Umfang:** implementieren + committen. **Kein Publish** — Release erst gemeinsam mit der
Folgerunde „lean-md scheitert leise". Berührt beide SemVer-Linien (Pack: `content/skills/**`;
Binary: `src/**` + `content/templates/**` via `include_str!`), aber **keine** Versionsnummer
wird in diesem Paket gezogen.

## Problem

Die 8 `content/skills/*/SKILL.md`-Stubs dokumentierten `ctx_md_render(skill=…, phase=…)`
als direkten MCP-Aufruf — in abtippfertiger Form. Der Aufruf kann nie gelingen: lean-md
ist ein eigenständiger stdio-MCP-Server (`lean-ctx-addon.toml`: `[mcp] command="lean-md"
args=["mcp"]`), erreichbar nur über den lean-ctx-Gateway. lean-ctx enthält keine Zeile
lean-md-Code.

Der fehlgeschlagene Aufruf liest sich wie ein toter Gateway und schickt Agenten für die
ganze Session in den Shell-Fallback. Der Fallback funktioniert und meldet keinen Fehler —
der Bug ist selbst-verschleiernd.

**Wurzel:** der `ctx_`-Präfix behauptet Zugehörigkeit zu einem Namensraum
(`ctx_read`/`ctx_search`/`ctx_shell` = direkte lean-ctx-Tools), in dem das Tool nicht
liegt. Ein Name, der eine falsche Architektur behauptet.

Korrekte Form (live verifiziert, und in `INSTALL.md` die ganze Zeit richtig):

    ctx_tools(action="call", tool="lean-md::ctx_md_render",
              arguments={"skill": "<name>", "phase": "<phase>"})

## Designentscheidungen

### 1. Stub-Straffung (Pack)

Der bloße Name `ctx_md_render` steht in keinem Stub mehr allein. Der volle Handle
`lean-md::ctx_md_render` erscheint genau einmal pro Datei, innerhalb von `ctx_tools`.
Damit braucht es **keine** Erklärung mehr, warum der Direktaufruf scheitert — es gibt
keine Falle, die man erklären müsste.

- `description` trägt **nur noch**, wofür der Skill gewählt wird. Render-Mechanik
  (`Render-on-invoke via …`, `phase-isolated for the −88…−95% token lever`, `Native lmd
  port of …`) entfällt: sie entscheidet nicht über die Auswahl, kostet aber in jeder
  Session Tokens im Skill-Index.
- Der Erklär-Absatz („`ctx_md_render` is NOT a direct tool — lean-md is a separate MCP
  server…") entfällt ersatzlos.
- Die Kurzform `ctx_md_render(skill=…, phase=…)` verschwindet aus Phasen- und
  Companion-Listen; dort stehen nur noch Namen. Kein Shorthand, keine Konvention.
- `Transport closed` → einmal wiederholen. Bleibt, einzeilig (P7: ehrlich, weil die
  Ursache unbekannt ist).

Zielform:

```
description: <nur Auswahl-Trigger + "Use when …">
---
# <skill> (delegation stub)

Body renders one phase at a time — never read it from disk:

    ctx_tools(action="call", tool="lean-md::ctx_md_render",
              arguments={"skill": "<skill>", "phase": "pre-context"})

Same call below, `phase` getauscht (Companion: `companion` statt `phase`).
On `Transport closed`: retry once.
```

### 2. `@dispatch` komponiert die `.ext` (Binary) — P4

**Ist-Zustand, aus dem Code belegt:** `fragments.rs:51` registriert `dispatch-contract`
als Built-in; `resolve()` prüft Built-ins zuerst und returned früh (`:59-61`), der jailed
File-Fallback ist für diesen Namen unerreichbar — und suchte ohnehin
`dispatch-contract.lmd.md`, nie `.ext.lmd.md`. `dispatch.rs:80` ruft
`resolve("dispatch-contract")`; der String `.ext` kommt in `src/` nirgends vor.
`seeds.rs:30` materialisiert die `.ext` über `install_skill()` (`skill_install.rs:121`)
nach `<project_root>/.lean-ctx/lean-md/` — und nichts liest sie je.

**Fix, lokal in `DispatchBridge::execute`:** nach `resolve("dispatch-contract")`
(`dispatch.rs:78-81`) die Datei `<jail_root>/.lean-ctx/lean-md/dispatch-contract.ext.lmd.md`
lesen, falls vorhanden, und an `contract_raw` anhängen — **vor** `render_body` (`:113`).
Damit nimmt die `.ext` an Placeholder-Substitution und `@include`-Auflösung teil, genau
wie der Built-in-Teil. Die Registry bleibt unberührt.

**Skip-if-inert:** angehängt wird nur, wenn der Inhalt nach Entfernen von HTML-Kommentaren
und Whitespace nicht leer ist. Der unveränderte Seed produziert damit byte-identische
Ausgabe wie heute (#498) — deshalb kippt kein bestehender Dispatch-Test.

**Seed** wird auf HTML-Kommentar-Guidance umgestellt (heute: 7 reine `#`-Zeilen, also
Markdown-Überschriften, die sonst in jedem Contract landen würden):

```
<!-- Dispatch-contract extension (project seed).
     Auto-composed by @dispatch after the built-in contract.
     Add project-specific subagent rules below. Empty by default. -->
```

Der `@include .lean-ctx/lean-md/dispatch-contract.ext`-Workaround entfällt ersatzlos.
Die `lmd-executing-plans`-Design-Spec behauptet die Komposition bereits — sie wird durch
diesen Fix erstmals korrekt und bleibt unverändert.

### 3. Tool-Alias `lmd_render` / `lmd_check` (Binary) — P6

`tool_defs` (`bin/lean_md.rs:283`) bekommt zwei zusätzliche Einträge; `cmd_mcp` matcht
beide Namen (`:492`, `:553`). `ctx_md_render`/`ctx_md_check` bleiben als Alias bestehen —
additiv, bricht nichts.

**Die Stubs nennen weiterhin `ctx_md_render` — nicht den neuen Namen.** Die Stubs liegen
im Pack, `lmd_render` entsteht im Binary, und die Linien sind entkoppelt: ein Konsument
darf Pack `0.2.1` mit Binary `0.2.0` fahren (`version_req = "^0.2"` erlaubt es; laut
`dev-readme.md` ist diese Divergenz der ausdrückliche Sinn des Schnitts). Ein Stub, der
`lmd_render` nennt, adressiert dort ein Tool, das das Binary nicht kennt → gescheiterter
Aufruf → Fehldiagnose „Gateway kaputt" → Shell-Fallback. Exakt der Bug, den dieses Paket
behebt, nur unter neuem Namen. `ctx_md_render` funktioniert auf beiden Binaries.

`lmd_render` ist ab sofort in `tools/list` sichtbar und in `README.md`/`INSTALL.md` der
empfohlene Name. In Pack-Content wandert er erst, wenn ein Binary **mit** dem Alias als
Mindestversion durchsetzbar ist — was `min_lean_ctx` nicht leistet (es pinnt lean-ctx,
nicht lean-md). Ein Mechanismus dafür fehlt; er gehört in die „scheitert leise"-Runde.

**SemVer-Notiz für das spätere Release:** additive Tools sind streng genommen ein Minor.
Entschieden wurde dennoch eine Patch-Version — in `0.x` ist Minor der Breaking-Slot,
additive Patches sind dort vertretbar. Bewusst so, nicht übersehen. Die endgültige Nummer
fällt beim Release, gemeinsam mit den Funden aus „scheitert leise".

### 4. Doku (weder Pack noch Binary — hängt an keiner SemVer-Linie)

`README.md` + `INSTALL.md` bekommen den Abschnitt „Warum `ctx_md_render` regelmäßig als
'Gateway kaputt' fehldiagnostiziert wird" samt Diagnose-Reihenfolge:

1. `ctx_tools(action="list")` → zeigt es `lean-md [stdio, enabled]`? Dann läuft alles,
   der Aufruf war nur falsch adressiert.
2. `Transport closed`? Einmal wiederholen — sporadisch, der Gateway respawnt.
3. Erst wenn der Server wirklich fehlt: Shell-Fallback (`lean_md_bin` +
   `LEAN_MD_SKILLS_DIR`). Gleiches Binary, byte-identische Ausgabe.

**Wichtige Trennung:** Punkt 3 gilt für **Konsumenten**. In diesem Dev-Repo ist der
Shell-Fallback der *reguläre* Weg (`CLAUDE.md`), weil der Gateway den lean-md-Katalog
zwar führt, das Tool dem Agenten aber nicht direkt reicht. Ohne diese Trennung baut die
Doku die nächste Falle: ein Agent liest „Fallback nur wenn der Server fehlt" und dreht
die Fehldiagnose um 180°.

### 5. Release-Runbook als Skill `lmd-releasing-lean-md` (Pack)

**Problem:** `docs/dev-readme.md` liegt in keinem Auto-Load-Pfad. Eine andere Session weiß
nicht, dass es die Datei gibt, und rekonstruiert die Choreografie aus dem Gedächtnis — bei
einer Reihenfolge, die erzwungen ist (Pack vor Addon, Tag vor `sync-manifest`), ist Raten
teuer. Das ist derselbe Fehlertyp wie der Haupt-Bug: die Information existiert, ist korrekt,
und wird nicht gefunden.

**Entscheidung:** neuer Skill `content/skills/lmd-releasing-lean-md/` mit dem Runbook aus
`dev-readme.md` als Phasen (Vorschlag: `preflight` → `pack` → `binary` → `addon` → `verify`).
`dev-readme.md` bleibt die Prosa-Quelle; der Skill ist der auffindbare, phasen-isolierte
Einstieg.

**Registrierung — der Kern der Entscheidung:** `SKILLS` (`skills.rs:17`) und
`INSTALLABLE_SKILLS` (`skill_install.rs:10`) sind zwei **unabhängige** Registries. `SKILLS`
steuert, was renderbar ist; `INSTALLABLE_SKILLS` steuert, was `install_skill()` als Stub
materialisiert — `skill_md()` (`:55`) lehnt jeden nicht gelisteten Namen ab. Beide sind
heute deckungsgleich, aber kein Test koppelt sie.

Der Skill wird **nur in `SKILLS`** eingetragen, **nicht** in `INSTALLABLE_SKILLS`. Folge:

- In diesem Repo sofort renderbar über den Debug-Fallback
  (`$CARGO_MANIFEST_DIR/content/skills`, siehe `dev-readme.md` „Lokal ohne Pack
  entwickeln") — also über den Aufruf, den `CLAUDE.md` ohnehin vorschreibt.
- Bei Konsumenten wird **nie ein Stub materialisiert**: kein `.claude/skills/`-Eintrag,
  kein Skill-Index-Eintrag, **null Token**. Er reist nur als wenige Bytes im Pack-Archiv.

Damit entfällt der Zielkonflikt mit Entscheidung 1 vollständig — Pack-Mitgliedschaft und
Installation sind entkoppelt.

**Verworfene Alternativen:**

- `.claude/skills/` — gitignored, Materialisierungs-Ziel von `install_skill()`. Kein Ort
  für gepflegten Content; würde bei jedem Install überschrieben.
- Pack-freier Root (`content/dev-skills/`) — bräuchte neue Mechanik (Pack-Scope,
  Resolver-Pfad, Gate-Abgrenzung). Unnötig, weil die Allowlist das Ziel bereits erreicht.
- **Skill via `include_str!` ins Binary** — würde den #727-Schnitt rückabwickeln
  (`AGENTS.md`, `fragments.rs`: Skill-Content lebt seit P3 im Pack, nicht im Binary;
  `test-first-core`/`brainstorm-gate` wurden damals genau deshalb in die
  `SKILL_INCLUDES`-Pack-Stage verschoben). Jede Runbook-Korrektur hinge dann an einem
  Binary-Release. Nutzen null, da die Allowlist genügt.
- **Runbook als Fragment, das andere Skills per `@include` konsumieren** — keiner der 8
  Skills braucht das Release-Runbook; nur wer releast, braucht es. Ein Fragment ohne
  Konsument wäre P4 ein zweites Mal: materialisiert und ungelesen.

## Tests (TDD, rot zuerst)

- `.ext` mit Regel → erscheint im Dispatch-Output nach dem Contract.
- Unveränderter Seed → Output byte-identisch zu „ohne `.ext`" (#498).
- `.ext` fehlt → unverändert.
- Jail-Escape greift weiterhin.
- `lmd_render`/`lmd_check` in `tools/list`; beide Namen dispatchen identisch.
- `skills.rs:1595` (`brainstorm_stub_description_carries_must_trigger`) prüft heute auf
  `ctx_md_render` im Stub → muss auf die neue Form nachgezogen werden.
- `lmd-releasing-lean-md` rendert jede Phase; `install_skill("lmd-releasing-lean-md", …)`
  ergibt `Err(NotFound)` — der Beweis, dass er Konsumenten nie erreicht.
- `pack_drift` nach `LEAN_MD_BLESS=1` grün.

## Umfang dieses Pakets: implementieren + committen, KEIN Publish

Dieses Paket endet beim Commit. **Kein `pack publish`, kein Tag, kein Addon-Republish** —
die „scheitert leise"-Runde (P1/P2/P3/P5) folgt unmittelbar und geht gemeinsam mit diesem
Paket raus. Ein Publish jetzt hieße, dieselbe Choreografie zweimal zu fahren und
Konsumenten zwei Updates in kurzer Folge zuzumuten.

| # | Schritt | Linie |
|---|---------|-------|
| 1 | Branch von `feat-lmd-v2`; Stub-Straffung + `.ext`-Fix + Alias + Doku; `cargo fmt`; `cargo nextest run` | — |
| 2 | `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` → `content/skills.sha256` | Pack |
| 3 | Commit (Code + Content + Doku) | — |

**Erwartet nach Schritt 3:** `pack_drift` in CI meldet, dass `content/skills/` nicht mehr
zum letzten publizierten Pack passt. Das ist **kein Fehler**, sondern genau die Funktion
des Gates: es erinnert an den Schnitt, der später kommt. Die Versionsnummern in
`lean-ctx-addon.toml` und `content/skills.ctxpkg-hash` bleiben in diesem Paket
**unangetastet** — sie werden erst beim tatsächlichen Release gezogen.

`TODO.md` ist im Working Tree modifiziert, gehört **nicht** zu diesem Paket (bestand
vorher) und bleibt aus den Commits heraus.

## Release-Choreografie (Referenz — erst nach „scheitert leise")

Wenn beide Pakete fertig sind, wird **einmal** released. Die Reihenfolge ist erzwungen,
nicht Geschmack:

1. Skills-Pack muss publiziert sein, **bevor** das Addon republished wird — der Resolver
   löst `version_req` depth-1 gegen den Registry-Index auf; ein unpublizierter Pack ist
   unsichtbar.
2. Der Tag muss den 5-Leg-Build ausgelöst haben, **bevor** `sync-manifest` die echten
   SHA-256 in `[artifacts]` zurückschreibt (Bot-Commit auf `feat-lmd-v2`, nicht auf einen
   Tag — Loop-Safety).
3. Das Addon-Pack darf erst **nach** dem `sync-manifest`-Commit gebaut werden, sonst
   pinnt es Platzhalter-SHAs.

| # | Schritt | Linie |
|---|---------|-------|
| 1 | `pack create --version <v>` → `content/skills.ctxpkg-hash` aus `manifest.json` (`integrity.content_hash`) → commit | Pack |
| 2 | `pack export --sign` → `pack publish --token ctxp_…` — **von Hand**, CI hat bewusst kein Token | Pack |
| 3 | `lean-ctx-addon.toml`: Version + Artefakt-URLs; Tag `v<v>` → 5-Leg-Build → `sync-manifest`-Bot-Commit | Binary |
| 4 | Addon-Pack (`kind=addon`) exportieren + publizieren | Binary |

Solange der Skills-Pack in `0.2.x` bleibt, ist `version_req = "^0.2"` **unangetastet**.
Die konkreten Versionsnummern werden beim Release festgelegt, nicht hier — welche Funde
aus „scheitert leise" mitkommen, entscheidet über Patch vs. Minor.

## Nachgelagert: Konsumenten

In `canfdchela` sind `.claude/skills/*/SKILL.md` lokal gefixt (Commit `8b0f4a8`).
`install_skill()` überschreibt sie bei jedem Install (`skill_install.rs:98`) — erst der
Upstream-Fix ist dauerhaft. Bis zum Publish bleibt der lokale Fix dort die Zwischenlösung;
danach `addon update` fahren.

## Bewusst NICHT in diesem Paket

Eigene Brainstorm-Runde, Arbeitstitel **„lean-md scheitert leise"** (Kandidat 0.3.0).
Roter Faden: das Tool tut etwas anderes als dokumentiert und sagt nichts.

- **P1 — `check` semantisch machen (größter Hebel).** `lean-md check` parst nur. Es sagte
  `lmd ok` zu: fehlendem `phase=`, `role=exec` (existiert nicht), unaufgelösten Vars,
  doppelten Phasen-Namen. Vier von fünf Bugs wären hier gefallen. Ein Autor mit grünem
  `check` hält seine Datei für korrekt — das ist der Vertrauensbruch. Minimalprogramm:
  Pflichtargumente, Enum-Validierung (`role`), doppelte Phasen-Namen, unbekannte Argumente.
- **P2 — doppelte `@phase`-Namen: harter Fehler statt Datenverlust.** Der zweite Block
  verschwindet spurlos; `--list-phases` zeigt den Namen einmal, `--phase X` rendert nur
  den ersten. Stiller Content-Verlust in einem Dokumentations-Tool.
- **P3 — `@dispatch brief=` wird akzeptiert und verworfen.** Der Parser schluckt es, der
  Renderer ignoriert es. Entweder rendern oder als unbekanntes Argument ablehnen; stilles
  Verwerfen ist die schlechteste Option.
- **P5 — `fragments.rs` Doc-Kommentar (Z. 1–3) stimmt nicht.** „files override/extend
  them" ist für Built-in-Namen falsch. Entweder Kommentar präzisieren **oder** Overlay vor
  Built-in prüfen — letzteres erlaubt projekt-spezifische Contracts, ist mächtig und
  breaking. **Konflikt-Notiz:** ginge P5 auf Overlay-vor-Built-in, würde der `.ext`-Fix
  aus Entscheidung 2 obsolet (die Datei liefe dann über die Registry statt über einen
  Sonderpfad in der Bridge). Entscheidung 2 ist bewusst als Zwischenschritt akzeptiert.
- **P7 — `Transport closed`.** Sporadisch, nicht argumentabhängig (Verdacht auf
  `consumer:"ai"` per Retest widerlegt). Braucht eine Reproduktion, bevor ein Issue
  Substanz hat.
