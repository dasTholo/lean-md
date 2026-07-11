# Design: lean-md-Doku an den aktuellen Stand anpassen

- **Datum:** 2026-07-11
- **Status:** approved (brainstorm) → bereit für lmd-writing-plans
- **Scope:** `README.md`, `INSTALL.md`, `docs/dev-readme.md` — reine Doku-Änderung, kein Code, keine Tests.

## Ziel

Die drei Doku-Dateien spiegeln nicht den aktuellen Stand:

- `README.md` trägt ein WIP-Banner, das behauptet, Skills seien „not yet possible" —
  faktisch laufen 8 eingebettete Skills (P3/#727 gelandet, Pack-Stage aktiv,
  `min_lean_ctx 3.9.6`).
- `README.md` beschreibt nur *einen* „pilot skill" und keinen Aufruf-Weg.
- `INSTALL.md` nennt einen Registry-Pfad (`lean-ctx addon add lean-md`), obwohl weder
  Addon noch Skills-Pack publiziert sind — nur der lokale Build funktioniert heute.
- `docs/dev-readme.md` widerspricht sich selbst: Kopf sagt „Not active yet / Until P3
  lands", ein späterer Abschnitt sagt „Zwei Release-Regime (seit P3, #727)".

**Leitgedanke:** Skills als funktionsfähig darstellen, aber keine falschen
Registry-Versprechen machen — der lokale Build ist der einzige live installierbare
Pfad.

## Nicht im Scope (YAGNI)

- Kein neuer Skill, keine Änderung an Skill-Bodies/Companions.
- Keine Änderung an `lean-ctx-addon.toml`, `docs/CONTRACT.md`, Code oder Tests.
- Kein Release, kein Pack-Bump, kein Tag.

## Faktenbasis (verifiziert)

- 8 Skills unter `content/skills/`: `lmd-brainstorm`, `lmd-writing-plans`,
  `lmd-writing-skills`, `lmd-test-driven-development`, `lmd-executing-plans`,
  `lmd-subagent-driven-development`, `lmd-dispatching-parallel-agents`,
  `lmd-finishing-a-development-branch`.
- Aufruf-Mechanik: `SKILL.md` ist ein Delegation-Stub; der Body wird phasenweise über
  `ctx_md_render(skill="<name>", phase="<phase>")` (bzw. `companion=`) gerendert.
  CLI-Äquivalent: `lean-md render --skill <name> --phase <phase> --consumer=ai`
  (Companion: `--companion <name>`).
- Install-Realität: Nur Path B (lokaler Klon) funktioniert. `@dasTholo/lean-md` und
  `@dasTholo/lean-md-skills` sind **nicht** in der Registry publiziert.
- P3/#727 ist gelandet (Commits `0875341` min_lean_ctx→3.9.6, `3c26b1c`, `0d0fd72`
  rebless drift hashes); Pack-Stage resolved vor Builtins.

## Änderungen im Detail

### 1 · README.md

1. **WIP-Banner entfernen.** Das `> ⚠️ **Work in progress.** …`-Blockquote am Anfang
   ersatzlos streichen.
2. **Abschnitt „Skills" neu fassen** (ersetzt den bisherigen Ein-Satz-Absatz zum
   „embedded pilot skill"):
   - Intro-Satz: lean-md bettet 8 Skills ein — native Ports der superpowers-Prozess-
     Skills — die phasenweise on-demand gerendert werden (Token-Lever −88…−95 %).
   - **Tabelle gruppiert nach Workflow-Stufe**, Spalten `Stufe | Skill | Zweck`:

     | Stufe | Skill | Zweck (Einzeiler) |
     |---|---|---|
     | Design | `lmd-brainstorm` | Idee → freigegebene Design-Spec |
     | Plan | `lmd-writing-plans` | Spec → token-effizienter `.lmd.md`-Plan |
     | Ausführen | `lmd-executing-plans` | Plan inline ausführen, Checkpoints |
     | Ausführen | `lmd-subagent-driven-development` | ein Implementer-Subagent pro Task, Zwei-Verdikt-Review |
     | Ausführen | `lmd-dispatching-parallel-agents` | ein Subagent pro unabhängiger Domäne, dann Konflikt-Scan |
     | Abschluss | `lmd-finishing-a-development-branch` | Branch integrieren: merge / PR / keep / discard |
     | Querschnitt | `lmd-test-driven-development` | RED→GREEN→REFACTOR vor Implementierung |
     | Querschnitt | `lmd-writing-skills` | TDD für Skills — Pressure-Test zuerst |

     > Zweck-Texte aus den `description`-Feldern der jeweiligen `SKILL.md` ableiten
     > (im Plan als Anker referenzieren), nicht frei erfinden.
   - **Unterabschnitt „Wie der Aufruf funktioniert"** mit beiden Ebenen:
     - *End-User-Einstieg:* im Agent-Host (z. B. Claude Code) triggern Skills
       automatisch über ihre `description`, oder man ruft `/lmd-<skill>` als
       Slash-Command auf; der Host-Agent fährt die Phasen ab.
     - *Render-Mechanik:* jede `SKILL.md` ist ein Stub; Body/Companion werden über
       `ctx_md_render(skill="<name>", phase="<phase>")` phasenweise geholt (CLI:
       `lean-md render --skill <name> --phase <phase> --consumer=ai`). Phasen-Isolation
       = die Ersparnis.
3. **Install-Verweis** an INSTALL angleichen: „from the registry (once listed)" nicht
   als funktionierender Pfad darstellen — lokaler Klon ist der Weg, Registry als
   „geplant" markieren (konsistent mit §2).

### 2 · INSTALL.md

1. **Path A (Registry)** als **„geplant / noch nicht gelistet"** kennzeichnen — Addon
   und Skills-Pack sind nicht publiziert. Als zukünftiger Pfad stehen lassen, nicht als
   aktuelle Anleitung.
2. **Path B (lokaler Klon)** als den aktuell funktionierenden Pfad hervorheben; die
   `cargo install --path .` + `lean-ctx addon add ./lean-ctx-addon.toml`-Schritte
   bleiben/werden verifiziert.
3. **Kurznotiz** ergänzen: Skills laufen heute übers lokal gebaute Binary
   (Debug-Fallback auf `content/skills/`); Pack-Distribution über
   `@dasTholo/lean-md-skills` steht aus.
4. Prerequisites-, Verify- und Troubleshooting-Abschnitte bleiben inhaltlich erhalten
   (nur Registry-Formulierungen anpassen).

### 3 · docs/dev-readme.md

1. **Veralteten Kopf** „> **Not active yet.** … Until P3 lands …" **streichen** — P3 ist
   gelandet.
2. **Preconditions(P3)-Sektion** („None of the above works until all four are green" +
   V1–V4-Tabelle + „V1 is the hard blocker") **entfernen**, da erfüllt.
3. Die **zwei überlappenden Release-Regime-Blöcke** — der englische oben und der
   deutsche „Zwei Release-Regime (seit P3, #727)" unten — zu **einem** konsistenten
   Abschnitt zusammenführen (eine Regime-Tabelle, ein „Skill-Content ändern"-Ablauf,
   ein „Lokal ohne Pack entwickeln"). Doppelten Inhalt entfernen, keinen Fakt verlieren.
4. Behalten: Drift-Gate, Version-Coupling, „What consumers see".

## Sprache

- README.md, INSTALL.md: bestehende Sprache (Englisch) beibehalten.
- docs/dev-readme.md: gemischt (EN/DE) wie bisher — beim Zusammenführen eine der beiden
  Sprachfassungen des Regime-Abschnitts als Basis wählen und konsistent halten
  (Empfehlung: Deutsch, da der neuere „seit P3"-Block deutsch ist).

## Akzeptanzkriterien

1. `README.md` enthält kein WIP-/„not yet possible"-Banner mehr.
2. `README.md` listet alle 8 Skills gruppiert nach Stufe und beschreibt beide
   Aufruf-Ebenen (Slash-Command/Auto-Trigger **und** `ctx_md_render`/CLI-Render).
3. `INSTALL.md` stellt den lokalen Build als funktionierenden Pfad dar; der
   Registry-Pfad ist klar als „geplant/noch nicht gelistet" markiert.
4. `docs/dev-readme.md` hat genau einen konsistenten Release-Regime-Abschnitt, keinen
   „Not active yet"-Kopf und keine erfüllte Preconditions-Sektion mehr.
5. Keine Änderung außerhalb der drei Dateien.

## Übergabe

Nach Spec-Freigabe → `lmd-writing-plans` erstellt den `.lmd.md`-Plan (Anker auf die drei
Dateien, Zweck-Texte aus `SKILL.md`-`description`-Feldern).
