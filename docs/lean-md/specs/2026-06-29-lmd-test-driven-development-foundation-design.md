# lmd-test-driven-development — Native lean-md Port + Skill-Platform-Fundament (Design-Spec)

Status: **Design (in Review)** · Datum: 2026-06-29 · Branch: `feat-lmd-v2`

**Position im Authoring-Layer-Port (Spec 1 von 3):**

Der gewählte Scope ist der vollständige native Port des superpowers-Authoring-Layers,
dekomponiert in drei sequenzielle Specs entlang des Abhängigkeitsgraphen:

```
test-driven-development   →   writing-skills   →   brainstorming
   (DIESER Spec, #1)            (Spec #2)            (Spec #3, Re-Anchor)
```

**Dieser Spec ist Nr. 1.** Er liefert zwei Dinge in einem: (a) die erste nativ portierte
lean-md-Skill `lmd-test-driven-development`, und (b) das **geteilte Skill-Platform-Fundament**,
das Spec #2 und #3 wiederverwenden. TDD steht zuerst, weil `writing-skills` (Spec #2) TDD als
`REQUIRED BACKGROUND` voraussetzt — und das Fundament muss mit der ersten Skill landen.

**Referenz-Specs (Abgleich §9):**

- `2026-06-29-lmd-brainstorm-design.md` — schwester-Spec (#3); liefert das Architektur-Vokabular
  (Kanal ①/②, D6/D8/D10 Materialisierung, Body-Override D7, Gates). DIESER Spec **baut** das
  Fundament, das jener Spec **konsumiert**; jener wird nach Spec #2 auf „konsumiert nativ" re-angeankert.
- `2026-06-26-lean-md-standalone-addon-design.md` — autoritative Baseline (zero-config, 30 Bridges,
  `skills.rs`/`seeds.rs`/`availability.rs`, lean-ctx-seitiger Skill-Installer entfernt → Install lebt in lean-md).

## 1. Ziel & Kontext

Die superpowers-`test-driven-development`-Skill wird als **nativer lean-md-Skill**
`lmd-test-driven-development` portiert: binary-embedded, **phasenweise** über
`ctx_md_render(skill, phase)` gerendert. Gleichzeitig wird die heute auf **eine** hartkodierte
Skill (`lmd-brainstorm`) zugeschnittene Maschinerie zu einem echten **Skill-Platform-Fundament**
generalisiert.

**Warum TDD-Inhalt + Fundament in einem Spec:** Das Fundament (Registry, Body-Override,
`skill install`, `ctx_md_render`-Skill-Verdrahtung, Coverage-Gate) hat **keinen** eigenen
user-sichtbaren Deliverable — es muss mit dem ersten echten Skill-Konsumenten landen (YAGNI:
kein Phantom-„Infra-Spec"). TDD ist dieser erste Konsument.

**Nicht-Ziel:** `writing-skills` (→ Spec #2); Companion-Render-Targets / Out-of-band-Maschinerie
(→ Spec #2, dort gebraucht); brainstorm-Re-Anchor (→ Spec #3); kein Eingriff in den render core
(`rushdown`/`evalexpr`).

### 1.1 Ausgangszustand (verifiziertes Code-IST)

- `src/skills.rs` — `skill_body()` ist ein **hartkodierter `match`** auf `"lmd-brainstorm"`
  (ein `include_str!`-Const). `render_skill(name, phase, consumer, crp, jail_root)` mit
  Phasen-Isolation via `capture_phase_bodies` (kein Cross-Phase-Leak). **Kein** Multi-Skill-Lookup,
  **kein** Body-Override.
- `src/phases.rs` — N-Phasen-Modell existiert (`parse_phase_name`, `render_with_phases`,
  `capture_phase_bodies`). „8 Phasen" war reiner Inhalt, keine Code-Grenze. ✅
- `src/fragments.rs` — `FragmentRegistry::with_builtins()` + `resolve(name, jail_root)` mit jailed
  file-fallback (HARD_RULES, DISPATCH_CONTRACT). Trägt `@include`. ✅
- `src/seeds.rs` — `materialize_contracts(project_root, contracts_dir)` schreibt **nur**
  `lang/tooling`-Seeds in den Projekt-Overlay (absent-only, idempotent). **Nicht** Skills/Scripts
  nach `~/.claude/skills/`.
- `src/availability.rs` — `COVERAGE: &[(step, directive, backing)]` + Gate, **brainstorm-hartverdrahtet**
  (Titel „Brainstorming-Pfad"). Keine `skill`-Dimension.
- `src/bin/lean_md.rs` — CLI `main()` dispatcht `render|check|mcp` per `match action`. MCP-`tool_defs()`
  exponiert `ctx_md_render` mit **nur** `path/content/consumer/crp` — **kein `skill`/`phase`**.
  → `render_skill()` ist **nirgends exponiert** (toter Code außer Unit-Tests).
- `content/core/{hard-rules,dispatch-contract}.lmd.md`, `content/core/_fragments/` (leer),
  `content/tooling/availability-audit.md` (brainstorm-Coverage-Matrix).
- Kein `claude_state_dir()`-Helper, kein `CLAUDE_CONFIG_DIR`-Handling. → Neubau.

## 2. Entscheidungen

| #   | Fork                       | Entscheidung |
|-----|----------------------------|--------------|
| E1  | Reihenfolge                | **TDD zuerst.** `writing-skills` setzt TDD als `REQUIRED BACKGROUND` voraus; das Fundament landet mit dem ersten Skill-Konsumenten. |
| E2  | Skill-Name                 | **`lmd-test-driven-development`** — 1:1-Spiegel des superpowers-Quellnamens. Konvention für den ganzen Layer: *lmd-Skills spiegeln den superpowers-Quellnamen mit `lmd-`-Präfix*. **Begründung Kollision:** `lmd-tdd` verworfen, weil `tdd` in lean-ctx = **Terse Data Density** / CRP-Modus (`LEAN_CTX_CRP_MODE=tdd`, `CrpMode::Tdd` — `crp_proto.rs:9/17`, `crp.rs` `tdd_schema`/`tdd_legend`/`<!-- crp:tdd -->`). Ausgeschriebener Name kollidiert nicht (grep-`tdd` trifft ihn nicht) und vermeidet Falsch-Geschwisterschaft mit `lmd-writing-skills`. |
| E3  | Render-Modell              | **Phasen-isoliert** (`red`/`green`/`refactor`/`rationalizations`); `ctx_md_render(phase=X)` rendert nur X. **Disziplin-Mitigation** (Phasen-Isolation würde Trip-Wires verstecken): kompaktes `test-first-core`-Fragment via `@include` in **jede** Phase. |
| E4  | Infra-Ansatz               | **B — schlanke Registry-Tabelle.** `skill_body()`-`match` → `SKILLS: &[(name, body)]`-Lookup. Companion-Spalte/Out-of-band-Targets bewusst **deferred → Spec #2**. Keine verfrühte `Skill`-Trait-Abstraktion. |
| E5  | Discipline-Fragment        | **`test-first-core`** (Iron Law + „Letter==Spirit" + Red-Flags), **skill-eigen** unter `content/skills/lmd-test-driven-development/_includes/`. Registriert als Built-in (flacher **globaler** Name; `@include` löst per Name auf — Ordner kosmetisch). Ordner-Konvention **`_includes/`** (umbenannt von `_fragments`, 1:1 zur `@include`-Direktive); `content/core/` bleibt skillübergreifenden Fragmenten vorbehalten. Name bewusst **nicht** `tdd-core` (grep-Kollision mit `tdd_schema`/`tdd_legend`). |
| E6  | Body-Override (D7)         | Übernommen aus brainstorm-Spec: `render_skill` prüft zuerst Projekt-Overlay, sonst embedded Const → lokale Phasen-Iteration ohne Recompile. |
| E7  | `skill install`-Heimat     | **In lean-md** (Baseline §2.2: lean-ctx-Installer entfernt). Opt-in = Invocation. |
| E11 | Install-**Scope**          | **Auswählbar, Default `--local`.** `lean-md skill install <name> [--global\|--local]`. **`--local`** (Default) → `<project_root>/.claude/skills/<name>/` (env-unabhängig, versionierbar, team-teilbar). **`--global`** → `claude_state_dir()/skills/<name>/` mit neuem `claude_state_dir()`-Helper (`CLAUDE_CONFIG_DIR` sonst `~/.claude`). **`CLAUDE_CONFIG_DIR` betrifft NUR das globale Ziel** — das lokale ist immer projekt-relativ. Ersetzt den bisherigen `CLAUDE_CONFIG_DIR=<repo>`-Umlenk-Trick. |
| E8  | `ctx_md_render`-Verdrahtung| **Skill+Phase exponieren** (heute toter `render_skill`): `tool_defs()`-inputSchema + `tools/call`-Branch + CLI `render --skill/--phase`. Ohne dies ist keine native Skill invocable — gehört ins Fundament. |
| E9  | `COVERAGE`-Dimension       | `(step, directive, backing)` → **`(skill, step, directive, backing)`**; Audit-Doc bekommt `lmd-test-driven-development`-Abschnitt; stale Pfad `rust/src/lmd/availability.rs` → `src/availability.rs` fixen. |
| E10 | Keyword-Coverage           | `description`/Body schreiben **„test-driven development" / „test-first" / „write the test first"** aus; das Acronym **„TDD" nur disambiguiert** im Body (z. B. „TDD (test-driven development)"), nie als nacktes Keyword (CRP-Verwechslungsschutz). |

Aus brainstorm-Spec **übernommen** (nicht neu entschieden): D6 (Materialisierung Schicht A global / B Projekt-Overlay), D8 (Overlay-Pfad `.lean-ctx/lean-md/` als hartkodierte Konvention, optionaler Env-Override), D10 (Opt-in = Invocation).

## 3. Architektur

### 3.1 Zwei Schichten

**Inhalt (die Skill):** `lmd-test-driven-development` als 4-Phasen-Body, jede Phase `@include test-first-core`.

**Fundament (geteilt, von Spec #2/#3 wiederverwendet):**

1. **Skill-Registry** — `SKILLS: &[(&str, &str)]`-Tabelle statt hartkodiertem `match`.
2. **Body-Override (D7)** — Projekt-Overlay-Auflösung in `render_skill`.
3. **`ctx_md_render`-Skill-Verdrahtung** — `render_skill` über MCP-Tool + CLI exponiert.
4. **`skill install`/`remove`** — Materialisierung von `SKILL.md` (+ Scripts, hier keine) nach
   `claude_state_dir()/skills/<name>/`.
5. **`COVERAGE` mit `skill`-Dimension** — Audit-Doc + Gate.

### 3.2 Seed-Layout (alles unter `content/`, alles embedded)

```
content/skills/lmd-test-driven-development/
  SKILL.md          ~ Stub: frontmatter (name, SDO-konforme description) + render-on-invoke-Hinweis;
                      embedded → von `skill install` rausgeschrieben
  body.lmd.md       ~ 4 @phase-Blöcke (red/green/refactor/rationalizations); embedded → ctx_md_render
  _includes/        ~ @include-Partials (kein eigenständiges Render-Target; Unterstrich-Konvention)
    test-first-core.lmd.md  + Iron Law + "Letter==Spirit" + Red-Flags; skill-eigen, registriert als
                              Built-in (flacher globaler Name); @include in jede Phase
```

**`_includes/`-Konvention (umbenannt von `_fragments`):** Verzeichnis für `@include`-Partials — 1:1
zur `@include`-Direktive (`bridges/include.rs`), Jekyll/Hugo-analog. Der **Fragment-Namespace ist flach
und global** (Key in `FragmentRegistry::with_builtins()`); der Ordner ist **kosmetisch** (nur `include_str!`-Pfad
+ Consistency-Gate). `test-first-core` ist **skill-eigen** (jede Skill hat ihren eigenen Disziplin-Kern) →
liegt beim Skill, nicht in `content/core/`. `content/core/` bleibt **skillübergreifenden** Fragmenten
(`hard-rules`, `dispatch-contract`) vorbehalten; das heute leere `content/core/_fragments/` entfällt.

### 3.3 Materialisierung & Auflösung (gem. brainstorm-Spec §3.3, D6/D7)

| Seed | Laufzeit-Ziel | Scope |
|------|---------------|-------|
| `skills/<n>/_includes/test-first-core.lmd.md` | bleibt im Binary (Built-in, flacher Name) | embedded/locked |
| `skills/<n>/SKILL.md` | **`--local`** (Default): `<project>/.claude/skills/<n>/` · **`--global`**: `<CLAUDE_CONFIG_DIR\|~/.claude>/skills/<n>/` — Discovery, opt-in (E11) | materialisiert |
| `skills/<n>/body.lmd.md` | bleibt im Binary **— oder Projekt-Overlay (D7)** | embedded, override-fähig |

**Auflösungs-Reihenfolge (D7):** `render_skill()` prüft zuerst
`<jail_root>/.lean-ctx/lean-md/skills/<n>/body.lmd.md` (jailed); existiert → Overlay gerendert
(Phasen-Isolation läuft auf der Overlay-Quelle), sonst der `include_str!`-Const.

**Zwei Pfade, zwei Jobs (Discovery vs. Render-Override — NICHT verwechseln):**

| | Discovery (Claude Code findet die Skill) | Render-Override (lean-md-intern) |
|---|---|---|
| Pfad | `.claude/skills/<n>/SKILL.md` (E11) | `.lean-ctx/lean-md/skills/<n>/body.lmd.md` (D7, oben) |
| Wer liest | die **Harness** → zeigt `/lmd-…` | **`render_skill`** beim `ctx_md_render`-Aufruf |
| Inhalt | dünner **Stub** (verweist auf `ctx_md_render`) | alternative **Body-Quelle** |

Claude Codes Anforderung „Skills liegen in `.claude/skills/`" ist durch den **Stub** (Kanal ②) erfüllt;
der schwere Body fließt über `ctx_md_render` (Kanal ①) aus embedded Const **oder** dem
`.lean-ctx/lean-md/`-Override. Der Override-Pfad ist **kein** Discovery-Pfad — Claude Code schaut dort nie
hinein; er folgt lean-mds eigener Overlay-Konvention (`seeds.rs::materialize_contracts`, gleiches `.lean-ctx/lean-md/`).

## 4. Komponenten

### 4.1 Body — die 4 Phasen (treuer Port von superpowers-TDD)

`@phase "name" … @phase-end`; `ctx_md_render(phase=X)` rendert nur Block X. Jede Phase startet mit
`@include test-first-core`.

| Phase | Inhalt (aus superpowers-`test-driven-development`) | `@include` |
|-------|-----------------------------------------------------|-----------|
| `red` | failing test first; **Verify RED (mandatory)** — Test muss korrekt fehlschlagen; Good/Bad-Beispiel | `test-first-core` |
| `green` | minimaler Code zum Bestehen; **Verify GREEN (mandatory)**; YAGNI, keine Über-Engineering | `test-first-core` |
| `refactor` | Aufräumen **nur unter grün** (Duplikate/Namen/Helper); keine neue Behavior | `test-first-core` |
| `rationalizations` | volle „Common Rationalizations"-Tabelle (Excuse\|Reality) + „Why Order Matters" + „When Stuck" + Verification-Checklist | `test-first-core` |

**`test-first-core`-Fragment (kompakt, in jede Phase):** der Iron Law
(`NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST` + „Delete means delete"), der Satz
**„Violating the letter of the rules is violating the spirit of the rules."**, und die kompakte
**Red-Flags-Liste** („Code before test", „Test passes immediately", „I'll test after", …).
So trägt **jede** isoliert gerenderte Phase die Disziplin-Trip-Wires.

**lean-ctx-Adaption (writing-skills „one excellent example"):** Beispielsprache **Rust/`cargo nextest run`**
statt TS/`npm test` — passend zum Host-Crate und zur Hardrule (nie `cargo test`, kein `&&`-Chaining,
`ctx_shell` statt bash). „Verify RED/GREEN" referenziert `ctx_shell "cargo nextest run"`.

### 4.2 SKILL.md-Stub (SDO-konform)

```markdown
---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---
```

`description` = **nur Trigger**, kein Workflow-Summary (writing-skills SDO: ein Summary verleitet
Agenten, die Skill *nicht* zu lesen). Keyword-Coverage („test-driven development", „test-first")
im Body, Acronym „TDD" nur disambiguiert (E10). Body = render-on-invoke-Hinweis auf
`ctx_md_render(skill="lmd-test-driven-development", phase=…)`.

### 4.3 Skill-Registry (`src/skills.rs`)

`skill_body()`-`match` → Lookup gegen `SKILLS: &[(&str, &str)]` mit Einträgen `lmd-brainstorm`
(bestehend) und `lmd-test-driven-development` (neu). Unbekannt → `None`. (Companion-Spalte erst Spec #2.)

### 4.4 Body-Override (`render_skill`, D7)

Vor dem embedded Const: jailed Read von `<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md`;
existiert → Overlay-Quelle, sonst Const. PathJail-gebunden (Spec §6, keine Eskalation außerhalb
`contracts_dir`).

### 4.5 `ctx_md_render`-Skill-Verdrahtung (E8)

- `tool_defs()`: `ctx_md_render`-inputSchema um `skill` + `phase` (beide optional) erweitern.
- `tools/call`-Handler: wenn `skill` gesetzt → `render_skill(skill, phase, consumer, crp, jail)`;
  sonst bestehender `do_render`-Pfad. Fehler `UnknownSkill`/`PhaseNotFound` → JSON-RPC `-32602`.
- CLI: `lean-md render --skill <name> [--phase <p>]` analog (neue Flags in `parse_render_flags`).
- **Byte-stabil (#498):** CLI- und MCP-Pfad rufen denselben `render_skill` → identisches Ergebnis.

### 4.6 `skill install`/`remove` (`src/skill_install.rs`, neu)

- **Scope-Auswahl (E11):** `cmd_skill` parst `install|remove <name> [--global|--local]`, Default `--local`.
  - **`--local`** (Default) → Ziel `<project_root>/.claude/skills/<name>/` (env-unabhängig). `project_root`
    = aktuelles Arbeitsverzeichnis (bzw. dessen Git-/Projekt-Wurzel).
  - **`--global`** → Ziel `claude_state_dir()/skills/<name>/`; neuer `claude_state_dir()`-Helper:
    `CLAUDE_CONFIG_DIR` sonst `~/.claude`. **Nur** dieses Ziel reagiert auf `CLAUDE_CONFIG_DIR`.
- `install`: `include_str!`-`SKILL.md` (+ Scripts, hier keine) → Ziel-Dir. Atomic, idempotent
  (absent-tolerant). `remove`: entfernt nur das lmd-eigene Skill-Dir im **gewählten** Scope.
- CLI-Wiring: neuer `"skill"`-Arm in `bin/lean_md.rs::main()` → `cmd_skill`.
- Opt-in = Invocation; der Scope-Flag ist ein **Ziel-Selektor**, kein Opt-in-Gate.

### 4.7 `COVERAGE` + Audit-Doc (E9)

- `availability.rs`: `COVERAGE` → `&[(&str, &str, &str, &str)]` = `(skill, step, directive, backing)`;
  bestehende Zeilen bekommen `skill="lmd-brainstorm"`. Neu: `lmd-test-driven-development`-Zeilen —
  minimal, da TDD direktiv-arm: `("lmd-test-driven-development", "red", "read", "ctx_read")`
  (Test/Impl ansehen). Test-Execution (`ctx_shell "cargo nextest run"`) ist **keine** registrierte
  Direktive → Gap-Liste mit Begründung „TDD ist Prosa-Disziplin, Test-Run via roher `ctx_shell`".
- `content/tooling/availability-audit.md`: `lmd-test-driven-development`-Abschnitt; stale Pfad fixen.

## 5. Wiring & Gates

**Wiring:** `src/skills.rs` (Registry + Body-Override), `src/skill_install.rs` (neu) + `lib.rs`/`mod.rs`-Export,
`src/bin/lean_md.rs` (`ctx_md_render` skill/phase + `skill`-Subcommand), `src/availability.rs` (skill-Dim),
`src/fragments.rs` (`test-first-core` als Built-in registrieren + Consistency-Gate auf neue Seeds),
`content/skills/lmd-test-driven-development/*` (inkl. `_includes/test-first-core.lmd.md`) + Audit-Doc.

**Gates (`cargo nextest run`, nie `cargo test`):**

1. **Phasen-Isolation** für alle 4 Phasen (`red`/`green`/`refactor`/`rationalizations`) — kein Cross-Phase-Leak (bestehendes Test-Muster erweitern).
2. **`test-first-core`-`@include`** resolved in **jeder** Phase (Iron-Law-Marker in jeder isolierten Phase präsent — Disziplin-Mitigation E3).
3. **Registry**: `skill_body()` löst `lmd-brainstorm` **und** `lmd-test-driven-development`; unbekannt → `None`.
4. **Body-Override (D7)**: Overlay-Datei vorhanden → Overlay gerendert; absent → embedded (tempdir + jail).
5. **`skill install`-Roundtrip beide Scopes** (tempdir): **`--local`** → `<tmp_project>/.claude/skills/<name>/` (env-unabhängig, ignoriert gesetztes `CLAUDE_CONFIG_DIR`); **`--global`** mit `CLAUDE_CONFIG_DIR`-Pin → `<pin>/skills/<name>/`. Je install → `SKILL.md` da; remove → weg; idempotent.
6. **`ctx_md_render` skill/phase**: MCP- und CLI-Pfad rendern identisch (byte-stabil); `skill`-Param branched korrekt; Fehler → `-32602`.
7. **Fragment-Consistency-Gate** grün: built-in == on-disk für alle neuen Seeds (body, test-first-core, SKILL.md).
8. **`COVERAGE`↔Audit-Doc** inkl. `skill`-Dimension; jede covered Direktive in `default_registry()` registriert.
9. **Determinismus (#498)**: keine Timestamps/Counter, byte-stabil; `CliBackend` == `McpBackend`.

## 6. Lokaler Test-Flow (in-Repo, kein globaler Eingriff)

1. Seeds + Registry + Body-Override + `skill_install` + `ctx_md_render`-Wiring + `COVERAGE`-Erweiterung schreiben.
2. `cargo nextest run --manifest-path Cargo.toml` → alle 9 Gates grün.
3. `cargo fmt` vor jedem `git add` (Standalone-Crate, `Cargo.toml`+`src/` im Repo-Root).
4. Manueller E2E: `lean-md skill install lmd-test-driven-development --local`
   → repo-lokales `.claude/skills/lmd-test-driven-development/` (Default-Scope, kein env-Trick nötig);
   `lean-md render --skill lmd-test-driven-development --phase red` rendert die isolierte RED-Phase.
5. `.lean-ctx/lean-md/` + repo-lokales `.claude/` in `.gitignore` (falls noch nicht).

## 7. Risiken & offene Punkte

- **R1 — Disziplin-Schwäche durch Phasen-Isolation:** bewusst gewählt (E3); mitigiert durch
  `test-first-core`-`@include` in jeder Phase (Gate 2). Restrisiko: ein Consumer rendert nur `green`
  und sieht die **volle** Rationalization-Tabelle (Phase `rationalizations`) nicht — akzeptiert, da
  die kompakten Red-Flags via Fragment präsent sind.
- **R2 — Port-Treue vs. net-neuer Skill (writing-skills Iron Law):** `lmd-test-driven-development` ist
  ein **Port einer upstream bereits pressure-getesteten Skill** — das „failing test first" war upstream
  erfüllt. Port-Risiko = Treue + Render-Korrektheit, abgedeckt durch §5-Gates. **Bootstrap:** Spec #1
  wird mit der *superpowers*-`writing-skills` autorisiert (die native `lmd-writing-skills` existiert erst
  nach Spec #2); `lmd-brainstorm` (Spec #3) wird durch die **native** Schleife re-autorisiert. Optionaler
  Pressure-Test der gerenderten Disziplin = empfohlen, nicht blockierend.
- **R3 — `claude_state_dir()` Neubau:** kein vorhandener Helper zum Spiegeln (anders als brainstorm-Spec
  annahm). Muss `CLAUDE_CONFIG_DIR` korrekt honorieren (v2/#596-Konvention); Test pinnt die Env.
- **R4 — `testing-anti-patterns.md`** (Companion der superpowers-TDD-Skill) ist **nicht** in Spec #1:
  Companion-Out-of-band-Maschinerie landet erst in Spec #2. Bis dahin: kurzer Inline-Pointer in der
  `rationalizations`-Phase, voller Port als Companion in Spec #2.
- **R5 — Naming-Konvention-Drift:** `lmd-brainstorm` ist leicht von `brainstorming` verkürzt (pre-existing),
  während die neue Konvention (E2) 1:1-Spiegelung verlangt. Vermerkt, **nicht** Teil dieses Specs
  (kein Rename bestehender Skills hier).

## 8. Scope-Abgrenzung

- **In Spec #1:** `lmd-test-driven-development` (4 Phasen + `test-first-core`), Skill-Registry,
  Body-Override D7, `ctx_md_render` skill/phase-Verdrahtung, `skill install/remove` + `claude_state_dir()`,
  `COVERAGE` skill-Dimension + Audit-Doc, alle 9 Gates.
- **→ Spec #2:** `lmd-writing-skills`; Companion-Out-of-band-Render-Targets + Maschinerie;
  `testing-anti-patterns.md`-Port; Companion-Spalte in der Registry.
- **→ Spec #3:** `lmd-brainstorm`-Re-Anchor auf native Konsumption.
- **Verworfen (YAGNI):** `Skill`-Trait-Abstraktion; Phantom-„Infra-only"-Spec.

## 9. Abgleich mit Referenz-Specs (Vollständigkeits-Check)

| Aspekt | In diesem Spec berücksichtigt? |
|--------|-------------------------------|
| `ctx_md_render(skill, phase)` render-on-invoke (brainstorm §1/§4.1) | ✅ §4.5 — **und** als heute fehlende Verdrahtung identifiziert (E8) |
| Phasen-Isolation ohne Cross-Phase-Leak (`capture_phase_bodies`) | ✅ §4.1, Gate 1 |
| `@include`-Fragment-Compose (`FragmentRegistry`) | ✅ §4.1 `test-first-core`, Gate 2/7 |
| Body-Override D7 (brainstorm §3.3) | ✅ §4.4, Gate 4 |
| Materialisierung Schicht A/B, Overlay-Pfad `.lean-ctx/lean-md/` (D6/D8) | ✅ §3.3 übernommen |
| `skill install` in lean-md, lean-ctx-Installer entfernt (Baseline §2.2) | ✅ §4.6 + R3 (`claude_state_dir()` Neubau) |
| zero-config, Opt-in = Invocation (D10) | ✅ §4.6 |
| `availability`-Coverage als prüfbares Gate (Baseline) | ✅ §4.7, Gate 8 — generalisiert um `skill`-Dim |
| Determinismus #498, byte-stabile Seeds, Fragment-Consistency-Gate | ✅ Gate 7/9 |
| writing-skills SDO (description = nur Trigger, kein Workflow-Summary) | ✅ §4.2, E10 |
| writing-skills „one excellent example", richtige Sprache | ✅ §4.1 Rust/nextest |
| writing-skills Iron Law auf den Port angewandt | ✅ R2 (Port-Treue-Reconciliation + Bootstrap) |

**Bewusste Erweiterung über die Baseline hinaus:** (1) Skill-Registry-Generalisierung,
(2) `ctx_md_render` skill/phase-Verdrahtung (E8, schließt toten `render_skill`-Pfad),
(3) `COVERAGE` skill-Dimension (E9), (4) Naming-Konvention + CRP-`tdd`-Kollisionsvermeidung (E2/E5).
Jede als solche markiert.
