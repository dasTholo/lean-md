# lean-md — Seed-Historie, Dedup und drei stille Fehler

**Status:** approved (Design)
**Datum:** 2026-07-17
**Vorgänger:** `docs/lean-md/specs/2026-07-17-lmd-scheitert-leise-design.md` (0.2.1, umgesetzt a7e722a..44ffab3)

## Motiv

Dasselbe wie bei 0.2.1: *das Tool tut etwas anderes als dokumentiert und meldet
nichts.* Vier der fünf Teile sind genau diese Fehlerklasse; das Dedup (B) ist ein
Terseness-Thema und reist mit, weil es dieselben Bytes anfasst.

## Befundlage (verifiziert 2026-07-17)

Drei materialisierte Seeds in `.lean-ctx/lean-md/` sind stale und tragen die
überholte Edit-Regel (`@edit` for non-symbol edits only), wo der embedded Seed
längst den anchored-Loop vorschreibt:

| Seed | lokal (sha256/16) | embedded | stammt aus |
|---|---|---|---|
| `tooling/mcp-tools.lmd.md` | `cfc737ebda49d529` | `e972fb69b92de3dd` | `18eff9b` |
| `lang/rust.lmd.md` | `48dd2f30a4461244` | `c2d80ee4f8cafbb5` | `ed3f5f6` |
| `plan-template.lmd.md` | `a9744f311b6841a1` | `b800db5634a0de2f` | `09d5006` |

`plan-recipes.lmd.md` ist **nicht** stale (lokal == embedded). Der Non-Goal des
Vorpakets sprach von „vier stale Seeds" — es sind drei.

`dispatch-contract.ext.lmd.md` ist **aktuell** und **inert**: ein echter
`self-review`-Render trägt keine Fremdzeilen mehr. Die Gotcha `p5-task-5`
(„Fremdzeilen im Dispatch") ist damit **überholt**, ebenso der dafür gelockerte
Non-Goal.

Jede der drei stale Kopien trifft exakt eine frühere `content/`-Version. Keine ist
ein Nutzer-Edit.

## Der Kernbefund: der Heal-Pfad ist für den Bestand tot

`.lean-ctx/lean-md.lock` existiert in diesem Repo **nicht**. Damit haben alle drei
stale Seeds unbekannte Provenienz und laufen in `seeds.rs:204` („Local edit, or no
lock entry: never overwrite"). Sie heilen **nie** — sie bekommen dauerhaft `.new`
und eine Meldung.

Das trifft nicht nur dieses Repo: **jede Installation, die vor P8 materialisiert
wurde, hat keinen Lock.** Der Heal-Pfad, den P8 gebaut hat, greift ausschließlich
für Neuinstallationen. Ein zweites Projekt (`canfdchela`) trägt dieselbe stale
`mcp-tools.lmd.md` — der Beleg, dass das kein Einzelfall ist.

## Teil A — Seed-Historie

### Entwurf

Die Historie lebt als **`content/seeds.sha256`** — ein checked-in Manifest im
`sha256sum`-Format, via `include_str!` eingebettet und zur Laufzeit geparst. Es ist
die **einzige** Quelle: `PROJECT_SEEDS` bekommt **kein** drittes Feld.

```
# lean-md seed history (#498)
# Every sha256 this seed has ever been shipped with, current one last.
# Append-only: LEAN_MD_BLESS=1 keeps prior lines and adds the new hash.
444f13764064b241…  lang/rust.lmd.md
48dd2f30a4461244…  lang/rust.lmd.md
c2d80ee4f8cafbb5…  lang/rust.lmd.md
```

Ein Seed heilt, wenn sein lokaler Hash **irgendeine** Zeile seiner Historie trifft;
der aktuelle embedded Hash ist per Definition die letzte davon. Der Eingriff in
`refresh_contracts` ist eine einzige Bedingung — der bestehende Heal-Zweig
(`seeds.rs:193`) lernt eine zweite Quelle für „das haben wir geschrieben":

```rust
if lock.get(&key) == Some(local_hex.as_str()) || history(rel).contains(&local_hex.as_str()) {
    // heal — unverändert
}
```

Der Lock bleibt die präzise Antwort, die Historie fängt den Bestand ohne Lock. Ack,
`.new` und `preserved` bleiben unberührt. Ein echter Nutzer-Edit trifft keinen
historischen Hash und ist weiterhin geschützt.

**Verworfen — Erst-Lauf adoptiert:** existiert kein Lock, die lokalen Hashes als
Provenienz eintragen. Das würde einen vor P8 gemachten Nutzer-Edit stillschweigend
als „unsere Bytes" adoptieren und beim nächsten Seed-Update überschreiben.

**Verworfen — Historie aus git zur Build-Zeit:** `build.rs` bräuchte git, bräche
bei Tarball-Installs und hinge die Seed-Bytes ans VCS. Verletzt #498.

**Verworfen — Historie als drittes `PROJECT_SEEDS`-Feld (Code) + Manifest als
Gate.** Das war der erste Entwurf; er ist am Bless gescheitert (siehe unten) und
hätte in der reparierten Form jeden Hash an zwei Stellen geführt — dieselbe
Redundanz, die Teil B bekämpft.

### Das Gate

Die Historie hat ein Pflege-Problem, und es ist exakt das, an dem das
Fragment-Gate gescheitert ist: `include_str!` ist build-invalidierend. Ändert
jemand einen Seed, kann kein `cargo`-Test die **alte** Version noch sehen — sie
existiert zur Compile-Zeit nicht mehr. Ein Gate, das hier X == X prüft, wäre genau
die Tautologie, die Teil D beseitigt.

`pack_drift` löst dasselbe Problem bereits und braucht dafür kein git: es
vergleicht `content/skills` gegen ein checked-in Manifest (`content/skills.sha256`).
Ein checked-in Manifest ist ein Snapshot, den der Compiler nicht mitzieht — im
Gegensatz zu `include_str!`.

**Der entscheidende Unterschied zu pack_drift: der Bless ist append-only, nicht
replace.** `pack_drift`s Bless *ersetzt* das Manifest. Übernähme `seeds.sha256`
diese Semantik, würde derselbe Befehl, der den Test grün macht, die Historie
löschen — und ein Bless allein ließe einen Seed ohne Historie-Eintrag durchgehen:
grün, und der Bestand heilt still nie. Genau die Fehlerklasse, gegen die dieses
Paket antritt. Der Bless **hängt an**; bestehende Zeilen werden nie entfernt.

Weil das Manifest die einzige Quelle ist, gibt es nichts synchron zu halten, und
das Gate kollabiert auf zwei schlichte Prüfungen:

1. **Vollständigkeit:** der aktuelle Hash jedes Seeds steht im Manifest. Wenn nein
   → rot: *„Seed `x` geändert. Blesse (append-only): `LEAN_MD_BLESS=1 cargo nextest
   run --test seed_history`."*
2. **Append-only:** keine Manifest-Zeile geht verloren. Der Bless-Pfad liest das
   bestehende Manifest, hängt an und schreibt nie weniger Zeilen zurück.

Prüfung 2 hat echte Zähne, weil sie die Datei gegen ihren eigenen Vorzustand hält —
den der Compiler nicht mitbewegen kann.

Für den Endnutzer ändert sich nichts: alles ist checked-in, `cargo install` aus
einem Tarball ohne git baut grün. git wird **einmalig** gebraucht, um die 23
historischen Hashes zu ziehen — danach nie wieder.

## Teil B — Dedup dispatch-contract ↔ hard-rules

`content/core/dispatch-contract.lmd.md` macht `@include hard-rules` und wiederholt
danach drei von dessen Regeln. Sichtbar in jedem `@dispatch`-Render.

Die drei Paare sind **nicht identisch** — das ist der Grund für einen
differenzierten Schnitt statt eines pauschalen Streichens:

| Contract-Zeile | Befund | Aktion |
|---|---|---|
| `Search → ctx_search … read files → ctx_read` | echt doppelt, hard-rules deckt es vollständig | **streichen** |
| `NEVER fresh, NEVER raw` | `fresh` fehlt in hard-rules ganz; `raw` steht dort nur bedingt (`unless compression is provably wrong`) und nur für `ctx_shell` | **behalten**, auf das Delta eingedampft |
| `Rust (*.rs) non-symbol edits → …` | redundant **und** eine stillschweigende Verengung: hard-rules gilt generisch, diese Zeile suggeriert „nur `.rs`" | **streichen** |

Das Delta der fresh/raw-Zeile ist **beides**: `NEVER fresh` (in hard-rules gar
nicht vorhanden) **und** `NEVER raw` als unbedingte Regel für `ctx_read` (hard-rules
deckt nur `ctx_shell raw=true`, und dort nur bedingt). Beide bleiben stehen, dazu
der Re-Read-Weg über `ctx_delta` / `mode=diff`. Wer hier nur `NEVER fresh` behält,
verliert eine Regel und verfehlt das Erfolgskriterium.

Ergebnis: kein Regelverlust. Die vier echt dispatch-spezifischen Zeilen
(`tool_profile=power`/ToolSearch/nie `ctx_call`, git commit plain,
CRP-Ausgabedisziplin, sowie das eingedampfte fresh/raw-Delta) bleiben.

Jede gestrichene Zeile ändert, was dispatchte Subagents lesen — Verhalten, nicht
Kosmetik. #498 hängt an jedem Contract-Byte, also eigene Tests.

## Teil C — Renderer-Fence-Awareness

Ein `@phase` in einem Code-Fence wird vom Renderer als echte Definition behandelt:
`--phase t` liefert den Fence-Inhalt, der echte Block verschwindet, und `check`
zertifiziert das als „ok". Nicht bloß ein false negative des Gates, sondern
**falscher Content als Phase** — die Fehlerklasse des Vorpakets, von dessen eigenem
Gate durchgewunken.

`duplicate_phase` ist bereits fence-aware (Helper `unfenced_lines`,
`phases.rs:468`); `render_with_phases` und die Capture-Pfade line-scannen ohne
Fence-Tracking. Die Knowledge führt diese Asymmetrie als bewusst („Gate
konservativer als der Renderer"). Sie wird aufgelöst: der Renderer stellt auf
dieselbe Quelle um, Gate und Renderer sagen dann dasselbe.

**Blast-Radius: null.** Ein Korpus-Sweep über alle `.lmd.md` (```-Fences **und**
eingerückte Code-Blöcke) findet **0 Dateien** mit gefenctem `@phase`. Der heutige
Korpus rendert byte-identisch; #498 ist bei diesem Fund nicht in Gefahr. Das
`@phase` in `plan-template.lmd.md` ist ein echtes, absichtlich ungefenctes
„rendered example".

**Anmerkung zur Helper-API:** `unfenced_lines` liefert nur die ungefencten Zeilen.
Der Renderer muß gefencte Zeilen **behalten** (als Text), nicht wegfiltern — er
braucht „ist Zeile N gefenct?", nicht „gib mir die ungefencten Zeilen". Die
Helper-API wächst entsprechend; `duplicate_phase` bleibt Konsument der bestehenden
Form.

**Der Fast-Path (`phases.rs:243-246`) scannt ebenfalls ohne Fence-Tracking** und
wird mit umgestellt. Bei Blast-Radius null ist er unkritisch, aber ihn stehen zu
lassen hieße, eine zweite Asymmetrie zu konservieren, während man die erste
beseitigt.

## Teil D — Die echte Tautologie in gloss.rs

`gloss.rs::embedded_table_matches_on_disk_file` vergleicht `GLOSS_TABLE_SRC` gegen
`std::fs::read_to_string(".../content/gloss/directives.lmd.md")` — also exakt die
Datei, aus der `include_str!` die const speist. X == X. Der Kommentar dort gibt es
zu („include_str! identity").

Ersatz nach dem Muster von `resolve_returns_each_seed_verbatim` (0.2.1, Task 10):
prüfen, was real brechen kann — die **Verdrahtung** Tabelle→Konsument (ein falsch
gemappter `include_str!`, ein Konsument, der den Text trimmt/umbricht/mutiert) —
nicht `include_str!` gegen sich selbst.

Anders als bei den Seeds gibt es hier keine `resolve()`-Indirektion, also auch
keinen Restwert der bisherigen Form.

## Teil E — `ack` ohne Argumente meldet erneut

`lean-md ack` meldet auf einem bereits quittierten Baum erneut `acked <pfad>`.
`ack_seeds` soll nur berichten, was die Quittierung tatsächlich verändert hat; ein
bereits quittierter Konflikt ist kein neues Ereignis. `unmatched` (der Nutzer fragt
nach etwas, das nicht in Konflikt steht) bleibt unverändert — das ist eine echte
Meldung.

`ack_seeds` setzt zudem `dirty = true` bedingungslos: ein No-Op-Ack schreibt heute
den Lock ohne Anlass. Gehört zum selben Fix — der Lock wird nur geschrieben, wenn
sich etwas geändert hat.

## Reihenfolge und Isolation

- **A vor E** — beide fassen `seeds.rs` an.
- **B, C, D** sind gegenseitig und von A/E isoliert (`content/core/`, `phases.rs`,
  `gloss.rs`).

## Erfolgskriterien

- Ein Refresh in diesem Repo heilt alle drei stale Seeds und legt einen Lock an —
  ohne `.new`, ohne Meldung.
- Ein echter Nutzer-Edit an einem Seed wird weiterhin preserved + `.new`.
- `content/seeds.sha256` wird rot, wenn ein Seed geändert wurde, ohne daß sein
  neuer Hash im Manifest steht; die Meldung nennt den Bless-Befehl. Ein Bless
  entfernt nie eine bestehende Zeile.
- Ein `@dispatch`-Render trägt jede Regel genau einmal und verliert keine —
  namentlich bleiben `NEVER fresh` und `NEVER raw` (für `ctx_read`) erhalten.
- Eine Datei mit `@phase "t"` in einem Fence vor dem echten `@phase "t"`: `--phase
  t` liefert den **echten** Block. (Daß `check` hier kein Duplikat meldet, ist
  bereits heute grün — `duplicate_phase` ist fence-aware; der Nachweis liegt allein
  auf dem Render.)
- Der heutige Korpus rendert byte-identisch (#498).
- `lean-md ack` auf quittiertem Baum ist still.
- Test-Suite grün, clippy sauber.

## Non-Goals

- Kein Publish, keine Versionsnummer.
- Kein Aufräumen der stale Seeds von Hand — Teil A liefert den Mechanismus, der
  sie heilt. Das ist der Nachweis, nicht die Umgehung.
- `docs/CONTRACT.md` (baumelnder Verweis aus AGENTS.md/CLAUDE.md/Cargo.toml) bleibt
  offen — eigenes Thema.
- P7 (Transport closed) bleibt draußen: kein Repro.
