# Spec: Crate komplett deutschfrei (content + src + tests)

**Datum:** 2026-06-30
**Branch:** lmd-bench
**Status:** Design genehmigt, bereit für Implementierungsplan

## 1. Ziel

Sämtliche deutschsprachigen Inhalte im Crate (`content/` ohne `content/skills`,
`src/`, `tests/`) auf Englisch umstellen — bei voller Funktions- und
Test-Erhaltung. Nach Abschluss enthält der Code-/Content-Bereich **kein** Deutsch
mehr: weder Kommentare noch gerenderte Output-Strings noch Test-Fixtures.

Begründung: Deutsch im Quellcode ist nicht zielführend (gemischtsprachige
Codebasis); der Output-Pfad (`consumer=human`, Gloss-Templates, CLI) soll
einheitlich Englisch sein.

## 2. Scope

**Innerhalb:** `content/gloss/`, `content/tooling/`, `content/core/`,
`content/lang/`, `content/templates/` (faktisch nur 2 Dateien sind deutsch),
`src/**`, `tests/**`.

**Außerhalb:**
- `content/skills/**` — explizit vom Nutzer ausgenommen.
- `docs/**` (inkl. der Spec-Quelle `docs/reference/…`) — die zitierten Spec-§
  bleiben dort unverändert; nur ihre deutschen Zitat-Titel in `src`-Kommentaren
  werden übersetzt, die `§`-Referenz selbst bleibt.

## 3. Betroffene Artefakte (4 Klassen)

### Klasse A — content-Dateien (Kern der Aufgabe)
| Datei | Inhalt | Hinweis |
|---|---|---|
| `content/gloss/directives.lmd.md` | Header-Kommentar + 27 Gloss-Templates | `include_str!`-eingebettet (`gloss.rs:11`); Gate `embedded_table_matches_on_disk_file` bleibt nach Recompile selbst-konsistent |
| `content/tooling/availability-audit.md` | reine Doku (menschenlesbare Projektion von `availability.rs::COVERAGE`) | nicht eingebettet; kein Test prüft die Strings |

### Klasse B — Output-rendernde Funktionen (Verhalten + Test ändern sich gekoppelt)
Jede Einheit ist **atomar**: Code/Template + zugehörige Test-Assertion müssen im
selben Schritt geändert werden, sonst rote Tests.

1. **Gloss-Templates** — `content/gloss/directives.lmd.md` ↔ `src/gloss.rs`
   Test `glosses_common_work_directives` (4 Assertions, Zeilen 108–123).
2. **human-legend** — `src/crp.rs::human_legend` (Zeilen 48–83) gibt deutsche
   Wörter aus + Doc-Kommentar „expanded to German words" (Zeile 46) ↔ Test
   `human_legend_expands_glyphs_to_words` (Zeilen 142–183). Verhaltensänderung
   des `consumer=human`-Branches (D-12). Nur lokal getestet, keine externen
   Snapshots (verifiziert).
3. **CLI-Strings** — `src/bin/lean_md.rs` user-facing Ausgaben (z. B.
   `eprintln!("{} existiert bereits — nicht überschrieben")`, Zeile 343) und
   evtl. weitere deutsche `eprintln!`/`println!`-Strings (Implementierung erfasst
   sie systematisch).

### Klasse C — Doc-/Code-Kommentare crate-weit
~50 `src`-Dateien mit deutschen `///`/`//!`/`//`-Kommentaren (u. a. `audit.rs`,
`bridges/dispatch.rs`, `bridges/handoff.rs`, `engine.rs`, `phases.rs`,
`render.rs`, `macros.rs`, `fragments.rs`). Keine Funktions-/Test-Kopplung — reine
Übersetzung. Spec-§-Referenzen bleiben; deutsche Zitat-Titel werden übersetzt.

### Klasse D — Test-Fixtures mit Umlauten
`src/render.rs` Test `splice_preserves_multibyte_prose_around_directive`
(Zeilen 389–401):
- Fixture `"Grüße {{ date }} — Größe äöü\n"` → `"Greetings ☃ {{ date }} — size ∆\n"`
- Assertion `out.starts_with("Grüße ")` → `out.starts_with("Greetings ")`

Der Test prüft ausschließlich Multibyte-Splice um eine Direktive; neutrale
Multibyte-Zeichen (`☃`, Em-Dash `—`, `∆`) erhalten den Testzweck ohne Deutsch.

## 4. Verbindliche Terminologie (Gloss-Mapping)

Gloss-Template und `gloss.rs`-Assertion müssen **wortgleich** sein.

| Direktive | Deutsch (alt) | Englisch (neu) |
|---|---|---|
| read | `Datei {0} lesen` | `Read file {0}` |
| search | `Suchen nach {0}` | `Search for {0}` |
| list | `Verzeichnis {0} auflisten` | `List directory {0}` |
| query | `Ausführen: {raw}` | `Run: {raw}` |
| find | `Semantische Suche: {raw}` | `Semantic search: {raw}` |
| symbol:refs | `Referenzen von {1} ermitteln` | `Resolve references of {1}` |
| symbol:def | `Definition von {1} finden` | `Find definition of {1}` |
| symbol:impl | `Implementierungen von {1} finden` | `Find implementations of {1}` |
| symbol:overview | `Symbol-Überblick von {1}` | `Symbol overview of {1}` |
| symbol | `Symbol-Analyse: {raw}` | `Symbol analysis: {raw}` |
| graph:dependents | `Abhängige von {dependents} ermitteln` | `Resolve dependents of {dependents}` |
| graph:callers | `Aufrufer von {callers} ermitteln` | `Resolve callers of {callers}` |
| graph:callees | `Aufgerufene von {callees} ermitteln` | `Resolve callees of {callees}` |
| graph | `Graph-Analyse: {raw}` | `Graph analysis: {raw}` |
| edit | `Code-Änderung anwenden` | `Apply code change` |
| repomap | `Repo-Karte erstellen` | `Build repo map` |
| impact | `Impact-Analyse für {0}` | `Impact analysis for {0}` |
| architecture | `Architektur-Überblick` | `Architecture overview` |
| outline | `Outline von {0}` | `Outline of {0}` |
| routes | `Routen auflisten` | `List routes` |
| smells | `Code-Smells prüfen` | `Check code smells` |
| review | `Code-Review` | `Code review` |
| inspect | `Inspektionen ausführen` | `Run inspections` |
| count | `Zählen: {raw}` | `Count: {raw}` |
| refactor | `Refactoring: {raw}` | `Refactor: {raw}` |
| reformat | `Code formatieren: {0}` | `Format code: {0}` |

Generischer Fallback (`unknown_directive_uses_generic_fallback`, `gloss.rs:128`):
`Direktive {@name}: {raw}` → `Directive {@name}: {raw}` (Template-Quelle prüfen;
falls in Code statt Tabelle, dort anpassen + Assertion).

**human-legend (`crp.rs`):**
`Funktion→Function`, `Klasse/Struct→Class/Struct`, `Trait/Interface` (bleibt),
`Typ→Type`, `Enum` (bleibt), `Wert/Konstante→Value/Constant`,
`öffentlich→public`, `asynchron→async`,
`**Verwendete Notation:** …` → `**Notation used:** …`.

## 5. Determinismus-Auflagen (#498)

- Keine Timestamps/Counter/Random in Output-Bodies einführen.
- `directives.lmd.md` ist `include_str!`-Seed: Übersetzung ändert eingebettete
  Bytes konsistent; Fragment-Konsistenz-Gate (built-in == on-disk) bleibt grün.
- `CliBackend`/`McpBackend` treffen denselben Handler → byte-identisch; keine
  divergierenden Übersetzungen.

## 6. Ansatz: Subagent-Fan-out (SDD-Kontrakt)

Ausführung über `superpowers:subagent-driven-development` unter dem
lean-ctx Multi-Agent-/Memory-Kontrakt (Controller prepend't Dispatch-Contract).

- **Klasse-B-Einheiten** je als **ein** Paket dispatchen (Code + Template + Test
  atomar), damit nextest pro Einheit grün bleibt.
- **Klasse-C-Dateien** nach Gruppen parallelisieren (reine Kommentar-Übersetzung,
  keine Kopplung).
- **Klasse A** geht in der jeweiligen B-Einheit auf (A1 mit Gloss, Audit-Doku
  separat).
- Pro Einheit: übersetzen → `cargo fmt` → `cargo nextest run` (Teilmenge).

Abgelehnt: skriptgestütztes `sed`-Mapping (fehleranfällig bei Prosa, Risiko bei
Output-Strings).

## 7. Verifikation (Definition of Done)

1. `cargo nextest run` — alle Tests grün (insb. `gloss`, `crp`, `render`,
   `availability`, `fragments`, `header`).
2. `cargo clippy` — 0 Warnungen.
3. Abschluss-Scan über `src/` + `content/` (ohne `content/skills`):
   - Umlaut-Scan `[äöüßÄÖÜ]` = 0 Treffer.
   - Deutsche-Funktionswort-Scan (`der|die|das|und|nicht|wird|…`) = 0 Treffer in
     Kommentaren/Strings.
   Nur neutrale Multibyte-Zeichen aus Klasse D dürfen verbleiben.
4. `cargo fmt --check` sauber.

## 8. Risiken & Gegenmaßnahmen

| Risiko | Gegenmaßnahme |
|---|---|
| Template/Assertion driften auseinander | Klasse-B-Einheit atomar; Terminologie-Tabelle §4 ist verbindlich |
| `consumer=human`-Output-Bruch | nur `crp.rs`-lokale Tests betroffen (verifiziert: keine externen Snapshots) |
| `include_str!`-Gate rot | Datei wird geändert → Recompile bettet neu ein; Gate selbst-konsistent |
| Übersehene deutsche Strings ohne Umlaut | Abschluss-Scan §7.3 mit Funktionswort-Liste, nicht nur Umlaute |
| Determinismus-Drift | §5; keine nicht-deterministischen Elemente |
