# Design-Spec: `.lmd.md`-Rohtext-Zugriff härten (`lean-md source` + Skill-Doku)

- **Datum:** 2026-07-04
- **Status:** Design, wartet auf Umsetzungsplan (lmd-writing-plans)
- **Betrifft:** lean-md CLI (`src/bin/lean_md.rs`), geteiltes Seed
  `content/core/hard-rules.lmd.md`, Skill `content/skills/lmd-writing-plans/body.lmd.md`,
  Projekt-Doku `CLAUDE.md` + `.claude/rules/subagent-multi-agent.md` (Cleanup)

## Motivation

Ein `/lmd-writing-plans`-Lauf sollte einen Plan schreiben, der `.lmd.md`-**Seeds**
selbst ändert (Recipes, Skill-Bodies). Der Author brauchte den **Rohtext** dieser
Seeds für exakte `old_string`-Edit-Anker — und fand keinen Pfad dorthin. Er
verfiel auf `ctx_read mode=full`, das die `.lmd.md` **renderte** (Bug 3:
`@import`-NotFound-Kaskade, `@define`-Makros konsumiert → Datei „wirkt fast leer"),
und wich per Trial-and-Error auf `mode=raw` aus — das ebenfalls nicht half.

### Empirische Befunde (Ursache)

1. In diesem Addon-Repo schickt `ctx_read` **jede** `.lmd.md` durch den lean-md-
   Renderer — in **allen** Modi. `mode=raw` unterdrückt nur die lean-ctx-
   Kompression, **nicht** das Rendering. Beleg: `mode=raw` auf
   `content/templates/plan-recipes.lmd.md` liefert
   `<!-- lmd: @import … failed: NotFound -->` plus leere Zeilen statt der
   `@define`-Makros.
2. Der lean-md-CLI kann nur `render | check | mcp | skill` — **kein**
   Rohtext-/`cat`-Verb.

∴ Es existiert **kein** Weg, den Rohtext einer `.lmd.md` zu lesen, ohne den
Renderer zu triggern (der bei Bug-3-`@import`/`@define` kollabiert). Native
`cat`/`Read` sind per Hard-Rule verboten.

## Problem-Präzisierung: zwei Zugriffs-Fälle

| Fall | Bedarf | Bestehender Pfad |
|------|--------|------------------|
| **A — `.lmd.md` als Konsum-Artefakt** | Plan-Task-Brief lesen; Recipe-Makro-API entdecken | `render --phase <p>` / `render --signatures` — vorhanden, korrekt |
| **B — `.lmd.md` als Editier-Ziel** | Rohtext eines Seeds/Templates/Skill-Bodies für exakte Edit-Anker | **keiner** — Lücke |

Fall A ist gelöst und dokumentiert. Die Härtung adressiert Fall B (fehlender
Rohtext-Pfad) **und** dokumentiert beide Fälle so, dass ein Agent nicht wieder in
`mode=full`/`mode=raw` läuft.

## Ziel

lmd-Skills gehen **nativ mit lean-md-Renderer-Mitteln** mit `.lmd.md`-Artefakten
um — pro Intent das richtige Mittel, inklusive eines echten Rohtext-Pfads für
Fall B. Kein Rückfall auf `ctx_read mode=full/raw` oder natives `cat`.

## Design

Vier Komponenten: ein neues CLI-Verb (schließt die Fall-B-Lücke), zwei
Content-Härtungen (dokumentieren die Zugriffs-Map dort, wo Agenten sie brauchen)
und ein abschließendes Doku-Cleanup (entfernt die dann redundanten Workaround-
Beschreibungen).

### 1. CLI-Verb `lean-md source <file.lmd.md>` (Code)

Neuer Rohtext-Pfad, der den Renderer **umgeht**:

- **Verhalten:** liest die Datei byte-identisch (`std::fs::read_to_string`) und
  gibt sie **unverändert** auf stdout aus. Kein Rendering, keine `@import`/
  `@define`/`@phase`-Verarbeitung, kein `--consumer`/`--crp`.
- **Verortung:** neuer Match-Arm in `main` (`src/bin/lean_md.rs:118–125`, neben
  `render|check|mcp|skill`) → `cmd_source(&args[1..])`.
- **Usage-Zeile** ergänzen:
  `source <file.lmd.md>   (raw file bytes, no rendering — for edit anchors)`
- **Fehlerfälle:** fehlendes Argument → `exit:1` mit Usage-Hinweis (wie
  `cmd_render`); nicht existierende Datei → `exit:1` mit Pfad im Fehler.
- **Determinismus (#498):** reine Funktion von (Dateiinhalt) — byte-stabil.
- **Aufrufform durch Skills:**
  `ctx_shell(command="cargo run -q --bin lean-md -- source <file.lmd.md>", raw=true)`
  (`raw=true`, weil der Rohtext nicht rekomprimiert werden darf).

Bewusst ein eigenes Verb (nicht `render --raw-source`): semantisch das Gegenteil
von `render` (Quelle vs. gerendert), konsistent mit dem Verb-Stil `check`/`skill`.

### 2. Geteilter Fakt in `content/core/hard-rules.lmd.md` (Content)

`hard-rules` wird via `@include` in **jeden** Skill gezogen — der Fakt härtet damit
brainstorm, writing-plans, writing-skills und tdd zugleich. Neuer Block
(voller Block, mit Ursache + Rohtext-Pfad, damit kein Nachschlagen nötig ist):

```
- A `.lmd.md` is a **rendered artifact, not a source file**. `@read`/ctx_read
  (any mode, `raw` included) and every `render` path RENDER it → `@import` NotFound
  cascade, `@phase` isolation collapse, `@define` macros consumed (the file looks
  "empty"). Access it with lean-md renderer means, per intent:
  - a task/phase brief                                   → `render --phase <p>`
  - the macro API index                                 → `render --signatures`
  - the raw source (copy shape / set exact edit anchors) → `lean-md source <file>`
  Never native cat/Read and never ctx_read for `.lmd.md` source — both render it.
```

### 3. Skill-lokaler Block in der `plan-format`-Phase (Content)

`content/skills/lmd-writing-plans/body.lmd.md`, `@phase "plan-format"` — die Phase
führt Template + Recipes bereits ein (`--signatures` steht dort schon). Neuer
Point-of-use-Block direkt danach:

```
**Reading the `.lmd.md` sources while authoring (see Hard Rules):** a plan,
template, recipe library or seed is a rendered artifact — `@read mode=full`|`auto`
AND `mode=raw` both render it (macros vanish, file looks empty). Access map:
- recipe macro API (`plan-recipes.lmd.md`)              → `render … --signatures`
- an existing plan / template phase brief               → `render … --phase <p>`
- raw source of a seed/template you must EDIT (anchors) → `lean-md source <file>`
```

### 4. Doku-Cleanup (nach Umsetzung der Komponenten 1–3)

Sobald `source` + die Seed-Härtung existieren, sind die ausführlichen Workaround-
Beschreibungen in der Projekt-Doku redundant — die Regel lebt nun im Seed
(`hard-rules`, via Render sichtbar) und der saubere Rohtext-Pfad ist ein CLI-Verb.
Entschlacken statt duplizieren:

- **`.claude/rules/subagent-multi-agent.md`** — der Abschnitt *„Plan brief = CLI
  phase render"* trägt die lange raw=true-Begründung und *„Never `ctx_read` a plan
  `.lmd.md`"*-Prosa. Auf das Nötige eindampfen: Fall-A-Render-Pfad (Brief =
  `render --phase`) bleibt als Kern; die Rohtext-Erklärung wird durch einen Verweis
  auf `lean-md source` + `hard-rules` ersetzt. **Nicht** löschen, was weiter gilt
  (Controller rendert Task-Briefs via `--phase`).
- **`CLAUDE.md`** — der Block *„Rendering lmd-skills (this dev-repo)"* bleibt für
  Fall A (Skill-Phasen via CLI rendern) gültig; ergänzt/verweist auf `lean-md
  source` für Fall B (Rohtext eines `.lmd.md`-Editier-Ziels), statt den Umweg über
  `ctx_read`/`raw` zu beschreiben.

Leitprinzip: die **normative** Regel steht künftig einmal im Seed; die Projekt-
Doku verweist darauf, statt sie zu wiederholen (DRY, kein Drift).

## Verifikation

- **CLI-Feature (`source`)** — TDD: neuer Test, der `lean-md source <fixture.lmd.md>`
  aufruft und assertet, dass die Ausgabe **byte-identisch** zur Quelle ist (inkl.
  `@import`/`@define`/`@phase`-Zeilen unverändert, kein NotFound-Kommentar). Ein
  Gegen-Assert zeigt, dass `render` derselben Datei die Makros konsumiert. `cargo
  nextest run` grün.
- **Content-Härtung** — Render-Gate + Fragment-Gate (kein neuer Test-Code):
  - `render --skill lmd-writing-plans --phase plan-format` zeigt den neuen Block.
  - `render --skill <any>` zeigt den erweiterten `Hard Rules`-Block (via `@include`).
  - `cargo nextest run` grün → Fragment-Konsistenz-Gate (#498, built-in ==
    on-disk-Seed) bleibt gehalten.
- **Doku-Cleanup** — manuelle Sichtprüfung: keine `.lmd.md`-Rohtext-Workaround-Prosa
  (`ctx_read`/`raw`-Umweg) mehr in `CLAUDE.md` / `subagent-multi-agent.md`; der
  Fall-A-Render-Pfad bleibt vorhanden; beide verweisen auf `lean-md source` bzw.
  `hard-rules` statt zu duplizieren.

## Non-Goals

- **Specs** (`docs/…/specs/*.md`) sind plain Markdown — kein Render-Problem,
  `@read` bleibt dort korrekt. Nur `.lmd.md` ist betroffen.
- **Kein** Eingriff ins lean-ctx-Addon-Routing (`ctx_read mode=raw` weiterhin
  rendernd) — das läge im lean-ctx-Repo; der neue CLI-Pfad umgeht es sauber.
- **Kein** Fix von Bug 3 selbst (`@import`-NotFound im Whole-Doc-Render) — separat
  getrackt; diese Spec macht den Rohtext trotz offenem Bug 3 zugänglich.

## Betroffene Dateien

- `src/bin/lean_md.rs` — `source`-Verb + `cmd_source` + Usage-Zeile
- (Testdatei nach Projektkonvention) — `source`-Rohtext-Test
- `content/core/hard-rules.lmd.md` — geteilter `.lmd.md`-Zugriffs-Fakt
- `content/skills/lmd-writing-plans/body.lmd.md` — `plan-format`-Block
- `.claude/rules/subagent-multi-agent.md` — Workaround-Prosa eindampfen (Verweis)
- `CLAUDE.md` — „Rendering lmd-skills"-Block um `lean-md source` (Fall B) ergänzen
