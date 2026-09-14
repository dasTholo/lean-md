# lean-md: `outline --json` — strukturierte Gliederung mit Prüfung — Design v1.0

**Stand:** 2026-09-14 · **Status:** beschlossen, nicht implementiert
**Anlass:** lean-herdr führt `.lmd.md`-Pläne mit Agents aus (lean-herdr-Spec
`docs/specs/2026-09-14-lean-herdr-plan-design.md`, dort „TP0“). Es braucht die Phasen eines
Plans, die `@call`s jeder Phase mit zerlegten Argumenten und eine Prüfung, ob jeder Aufruf
ein definiertes Makro mit passender Argumentzahl trifft — bevor ein Worker eine Phase
rendert. Heute gibt lean-md nichts davon strukturiert aus, und ein Parser außerhalb von
lean-md würde `split_call_args` nachbilden.
**Bezug:** `docs/RELEASING.md` (Fall „binary-only“),
`docs/lean-md/2026-08-31-distributionskanal-entfallen.md`
**Betrifft:** neu `src/outline.rs`, `tests/outline.rs`; geändert `src/phases.rs`,
`src/bin/lean_md.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `README.md`.

---

## 1. Ausgangslage

| Befund | Beleg |
|---|---|
| Kein Syntaxbaum, der einen Render überlebt; ein Knoten trägt `name`, rohe `args`, Byte-Spanne | `src/engine.rs:229-236`, `src/node.rs:12-19` |
| Struktur-Direktiven sind Zeilen-Scans in fester Reihenfolge | `src/engine.rs:200-222` |
| `iter_phase_blocks` liefert (Name, Rohtext) in Dokument-Reihenfolge; ein doppelter Name leert die Liste still | `src/phases.rs:608`, `:647-681` |
| Fenced Code wird bei Phasen übersprungen | `src/phases.rs:551` |
| `@call`-Argumente: Komma außerhalb Quotes trennt, ein äußeres Quote-Paar fällt weg, fehlende Argumente werden `""`, überzählige still verworfen | `src/macros.rs:131-175`, `:185` |
| Signatur eines `@define`: erstes `(` bis letztes `)`; ein späteres `@define` gleichen Namens gewinnt | `src/macros.rs:42`, `:89-107` |
| `@import` löst gegen das Arbeitsverzeichnis auf und übernimmt nur Makros | `src/macros.rs:457-470` |
| Unbekanntes Makro fällt erst beim Rendern auf; in einer Phase bricht es mit `PHASE_ABORTED` ab | `src/bridges/call.rs:36-42`, `src/phases.rs:436-473` |
| `render --list-phases` rendert nichts, löst keine Imports; Ausgabe `name<TAB>title` | `src/bin/lean_md.rs:275-321`, `src/phases.rs:704-720` |
| `render --signatures` löst Imports, Ausgabe Text | `src/macros.rs:476` |
| `check` prüft nur `@dispatch`-Argumente und doppelte Phasen, Ausgabe Text | `src/bin/lean_md.rs:132-177`, `src/arg_schema.rs:28-33` |
| Ein CLI-Render einer Phase ruft `ctx_session` | `src/phases.rs:392` |
| Einzige JSON-Abhängigkeit ist `serde_json` | `Cargo.toml:23` |

## 2. Ziel

Ein Unterbefehl, der ohne Rendern, ohne Bridges und ohne Seiteneffekte die Gliederung eines
`.lmd.md`-Dokuments und alle Makro-Befunde als ein JSON-Objekt ausgibt.

## 3. Aufruf

```
lean-md outline <datei | -> --json [--require-phase <name>[,<name>…]]
```

- `-` liest das Dokument von stdin.
- Imports werden wie beim Rendern gegen das aktuelle Arbeitsverzeichnis aufgelöst.
- `--json` ist Pflicht; eine Textausgabe gibt es nicht.
- `--require-phase` meldet jede genannte, fehlende Phase.

## 4. Ausgabe

Genau ein JSON-Objekt auf stdout:

```json
{
  "phases": [
    {"name": "task-1", "title": "Task 1: …", "line": 42,
     "calls": [{"macro": "route", "args": ["implement", "core", "app/models.py tests/test_models.py"], "line": 43}]}
  ],
  "macros": {"lane": ["name", "deps"], "route": ["work", "lane", "files"]},
  "errors": [
    {"kind": "unknown_macro", "line": 88, "phase": "task-2", "message": "…"}
  ]
}
```

| Feld | Bedeutung |
|---|---|
| `phases[]` | jede Phase in Dokument-Reihenfolge, auch bei doppelten Namen |
| `phases[].title` | dieselbe Regel wie `phase_title` |
| `phases[].line` | 1-basierte Zeile des `@phase` |
| `phases[].calls[]` | jedes aktive `@call` der Phase in Reihenfolge, außerhalb von Fenced Code; `args` aus `split_call_args` **ohne** Auffüllen oder Kürzen |
| `macros` | alle Makros aus dem Dokument und seinen Imports mit Parameterliste, nach Name sortiert; bei gleichem Namen gilt die Definition, die auch beim Rendern gewinnt |
| `errors[]` | Befunde, sortiert nach `line`, dann `kind`; der Schlüssel `phase` fehlt, wenn ein Befund zu keiner Phase gehört |

**Fehlerarten**

| `kind` | Wann | `phase` |
|---|---|---|
| `unknown_macro` | ein `@call` irgendwo im Dokument trifft kein Makro | falls in einer Phase |
| `arity` | Argumentzahl des Aufrufs ≠ Parameterzahl des Makros; `name()` zählt als null Argumente | falls in einer Phase |
| `duplicate_phase` | ein Phasenname kommt mehrfach vor (Zeile des zweiten Vorkommens) | Name |
| `import` | ein `@import` lässt sich nicht auflösen | — |
| `missing_phase` | eine Phase aus `--require-phase` fehlt (`line` = 0) | Name |

**Determinismus (#498):** Die Ausgabe ist eine reine Funktion aus Dokument, Imports und
Argumenten — keine Zeitstempel, keine Zähler, feste Reihenfolge. Zwei Läufe sind
byte-identisch.

## 5. Exit-Codes

| Code | Lage |
|---|---|
| 0 | JSON ausgegeben, `errors` leer |
| 1 | JSON ausgegeben, `errors` nicht leer |
| 2 | unbrauchbare Eingabe (Datei fehlt, `--json` fehlt, unbekanntes Flag); Meldung auf stderr, kein JSON |

## 6. Seiteneffekte

Keine: keine Bridges, kein `ctx_session`, kein Seed-Refresh, keine Datei wird geschrieben.

## 7. Umsetzung

- `src/outline.rs`: baut die Ausgabe aus `iter_phase_blocks` (um Startzeilen erweitert),
  `parse_directive_line` (`src/parser/block.rs:15-34`), `split_call_args` (vor dem Auffüllen),
  `parse_call_signature` und `extract_definitions` (Import-Fehler sichtbar machen). JSON über
  `serde_json::Value`; Objektschlüssel sortiert, Listen in definierter Reihenfolge.
- `src/phases.rs`: Startzeile je Phase; eine Variante, die bei doppelten Namen alle Blöcke
  liefert, ohne das bisherige Verhalten von `iter_phase_blocks` zu ändern.
- `src/bin/lean_md.rs`: Unterbefehl `outline`, Hilfe- und Usage-Text.
- `src/lib.rs`: Export von `outline`.

## 8. Tests

- `tests/outline.rs` nach dem Muster von `tests/list_phases.rs` (`env!("CARGO_BIN_EXE_lean-md")`
  + Temp-Verzeichnis): gültiger Plan; jede Fehlerart; stdin; `--require-phase`; `@call` in
  Fenced Code wird ignoriert; Komma in Quotes; `name()` mit null Argumenten; Import relativ zum
  Arbeitsverzeichnis; Exit-Codes 0, 1, 2; zwei Läufe byte-identisch.
- Unit-Tests in `src/outline.rs` für Zeilennummern, Sortierung und die Auflösung doppelter
  Makros.
- Ein Test belegt, dass `outline` kein `ctx_session` und keine Bridge berührt
  (`RecordingBackend`-Muster, `src/phases.rs:893-925`).

## 9. Arbeitsregeln

- Direkt auf `feat-lmd-v2`, keine Worktrees.
- Tests nur mit `cargo nextest run`; `cargo fmt` vor jedem `git add`; null clippy-Warnungen.
- Keine Shell-Ketten mit `&&`, `||` oder `;`.
- Verifikation nur über `cargo run -q --bin lean-md -- outline …`, nie über den Shim auf PATH.

## 10. Release (binary-only, 0.2.4)

1. Vorbereitungs-Commit: `Cargo.toml` und `Cargo.lock` auf 0.2.4, `CHANGELOG.md` mit
   `## [binary 0.2.4]`, README-Abschnitt zu `outline`. Seeds unberührt, kein Bless.
   `cargo nextest run` grün, insbesondere `determinism`, `seed_history`, `version_gate`.
   Hier endet die Arbeit eines Agents.
2. Maintainer: `git tag v0.2.4`, `git push --tags`; Release-CI und `sync-manifest`; `git pull`.
3. `lean-ctx addon publish` entfällt (Kanal entfernt).
4. Lokale Installation wie bei 0.2.3: GitHub-Release-Asset nach
   `~/.local/share/lean-ctx/addons/bin/lean-md/0.2.4/`, in `~/.config/lean-ctx/config.toml`
   `command` und `binary_sha256` des Blocks `lean-md` anpassen, lean-ctx neu starten.

## 11. Nicht-Ziele

- kein MCP-Tool für `outline`
- keine Änderung an `render`, `check` oder den Seeds
- kein Rendern ohne Bridges (`--no-work`)
- keine Textausgabe von `outline`

## 12. Abnahme

- `cargo nextest run` grün.
- Aus `/home/tholo/Scripts/lean-herdr` liefert
  `cargo run -q --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml --bin lean-md -- outline docs/lean-md/plans/2026-09-14-lean-herdr-rollen-und-routing.lmd.md --json`
  die Phasen `task-1` bis `task-9`; jeder Eintrag in `errors` ist von Hand als echter Fehler
  im Plan bestätigt.
- Nach der Installation liefert der Shim `lean-md outline … --json` dasselbe.
