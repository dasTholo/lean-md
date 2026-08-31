# Der lean-ctx-Addon-Kanal ist entfallen — Befund, lokaler Fix, offene Punkte

**Stand:** 2026-08-31 · **Anlass:** Release binary 0.2.3 · **Status:** lokal gelöst,
Distribution offen

Beim letzten Schritt des 0.2.3-Release (`lean-ctx addon publish --namespace dastholo`)
antwortete lean-ctx mit `Addon and plugin commands have been removed.` Dieses Dokument
hält fest, was die Untersuchung ergeben hat, was lokal getan wurde und was noch offen
ist.

## 1. Befund

Das Addon-/Plugin-System von lean-ctx ist **upstream entfernt**, nicht nur in einem
lokalen Build:

```
de96cc5b4  Yves Gugger  Fri Aug 21 18:19:04 2026   (in upstream/main)
refactor: remove dead modules (~22K LOC cleanup)

Removed: addons, plugins, neural, buddy, stigmergy, pair_protocol,
graph_expand, tui, science_*, information_bottleneck, free_energy_budget,
seed_observatory. All references cleaned from 47 source files.
```

- Implementierungs-Stub: `rust/src/cli/dispatch/mod.rs` → `eprintln!("Addon and plugin
  commands have been removed.")`, exit 1.
- `rust/src/cli/completions/spec.rs` führt den `addon`-Knoten als `hidden: true` mit
  `description: "Unavailable: marketplace/addon surface is Research"` und nur noch
  `list|install|remove|info` — **kein `publish`**.
- Verifiziert an drei Binaries: lokal gebautes 3.10.1, offizielles Release 3.10.0
  (= crates.io latest; ein 3.10.1 ist nirgends veröffentlicht) und offizielles 3.9.20
  (26.08.). Alle drei antworten identisch.
- Die 3.10.0-Release-Notes bestätigen es als Produktentscheidung: *"This is a local
  Research substrate, not a hosted registry, marketplace, automatic installer."*

lean-md 0.2.2 erschien am 19.07., als der Kanal noch lief. Die Bruchstelle liegt also
zwischen 0.2.2 und 0.2.3.

### Was das trifft

| Bereich | Status |
|---|---|
| Release 0.2.3 selbst | **intakt** — Tag `v0.2.3`, fünf Assets, `[artifacts.*]` von der CI gesynct und Hash-für-Hash gegen `SHA256SUMS` verifiziert |
| Renderer / `CodeIntelBackend` | **unberührt** — ruft `lean-ctx call <tool>`, das existiert weiter |
| `docs/RELEASING.md` Schritte 6–7 | **nicht ausführbar** — `addon publish` und `pack publish` gibt es nicht mehr; `lean-ctx pack` ist in 3.10.x ein PR-Context-Pack plus `checkpoint-seal/-inspect`, nicht mehr der ctxpkg-Paketmanager |
| README / INSTALL | **falsch** — bewerben seit `73cac4b` `lean-ctx addon add dastholo/lean-md` als *primären* Installationsweg; funktioniert seit 3.9.20 für niemanden |
| Skills-Auslieferung | **die eigentliche Baustelle** — siehe §3 |

## 2. Lokaler Fix (erledigt, verifiziert)

0.2.3 wurde von Hand installiert. Bewusst das **GitHub-Release-Asset**, nicht ein
lokaler Build — so läuft lokal exakt das Artefakt, das auch Konsumenten bekommen.

1. `lean-md-x86_64-unknown-linux-gnu` nach
   `~/.local/share/lean-ctx/addons/bin/lean-md/0.2.3/`, `chmod 755`.
   Der Shim `~/.local/bin/lean-md` findet es selbst — er nimmt
   `find … -name 'lean-md-*' | sort -V | tail -1`.
2. Gateway-Wiring in `~/.config/lean-ctx/config.toml`, Block
   `[[gateway.servers]] name = "lean-md"`: **`command` und `binary_sha256`** auf 0.2.3
   (`3eb9787e94aca2374a1e069a4d7c752c1201f70b86e5e56d3fe60a37c57a1297`). Beide sind
   nötig — der Gateway verweigert den Spawn bei Hash-Mismatch.
   Backup: `config.toml.bak-vor-lean-md-0.2.3`.
3. Ein lean-ctx-Neustart lässt den Gateway die neue Config lesen.

Verifikationen: Shim zeigt die 0.2.3-Usage (trennt `render <file> --phase P` von
`render --skill`; 0.2.2 hatte beides in einer Zeile) · sha256 des installierten
Binaries gegen den Pin: MATCH · Render über CLI **und** über
`tools/call ctx_md_render skill=… phase=…` gegen das installierte Binary: beide korrekt.

> **Werkzeug-Notiz für die nächste Session:** `sed`, `cat`, `ls`, `find`, `sha256sum`
> stehen nicht in der lean-ctx-Shell-Allowlist, und `ctx_patch`/native `Read` verweigern
> Pfade außerhalb des Projektroots. Out-of-root-Edits gehen über ein `python3`-Skript
> (inline `-c` ist gesperrt, Skriptdateien sind erlaubt); Verzeichnislisten über
> Shell-Globbing (`echo dir/*/`), Dateiinhalte über `grep "" datei`. Ein
> Änderungsskript sollte abbrechen, wenn ein Muster nicht genau einmal trifft.

## 3. Der Skills-Pack — heute in Ordnung, morgen ohne Weg

**Heute funktionsfähig, belegt:**

- Installiert unter `~/.local/share/lean-ctx/packages/skills/@dastholo__lean-md-skills/`:
  `0.0.0-precommit`, `0.2.0`, `0.2.1`. Shim und Gateway-`env` zeigen auf `0.2.1`.
- `diff -r -q` zwischen dem installierten Pack und `content/skills/` im Repo ist
  **leer** — kein Drift, der installierte Stand ist der aktuelle.

**Was fehlt:** der Update-Weg. `pack create/export/publish` existiert nicht mehr. Ändert
sich `content/skills/`, gibt es keinen Mechanismus, ein neues Pack auszuliefern oder zu
installieren — das Verzeichnis müsste von Hand befüllt werden.

Verschärfend: `LEAN_MD_SKILLS_DIR` wurde vom Addon-System aufgelöst
(`{pack_dir:@dastholo/lean-md-skills}` im Manifest). Im Release-Binary ist ein fehlendes
`LEAN_MD_SKILLS_DIR` ein **harter Fehler** — der Fallback auf
`$CARGO_MANIFEST_DIR/content/skills` ist `cfg(debug_assertions)` und damit nur im
Debug-Build aktiv. Ein Neu-Installierender hat also weder Pack noch Fallback.

## 4. Offene Punkte

1. **Antwort des lean-ctx-Maintainers abwarten** (Anfrage läuft): Kommt der
   Addon-Kanal zurück, oder ist die Entfernung endgültig? Die Antwort entscheidet, ob
   die folgenden Punkte nötig sind.
2. **Auslieferungsweg für `content/skills/`** — der Kern. Denkbar: die Skills in den
   Release-Tarball legen, `LEAN_MD_SKILLS_DIR` aus einem konventionellen Pfad neben dem
   Binary auflösen, oder den Debug-Fallback zu einem allgemeinen Suchpfad ausbauen. Das
   ist ein Design-Thema, kein Handgriff.
3. **README / INSTALL korrigieren** — sie beschreiben einen Befehl, den es nicht gibt.
   Unabhängig von Punkt 1 dringend, weil jeder neue Nutzer darauf aufläuft.
4. **`docs/RELEASING.md`** — die Fälle „skill-only" und „binary + pack" haben keinen
   Kanal mehr; „binary-only" endet ab jetzt beim GitHub-Release.
5. **`min_lean_ctx`** steht auf `3.9.6`. Für 0.2.3 bewusst so gelassen (Paket B ist der
   Degradations-Pfad für ältere lean-ctx; die `@read`-Anker brauchen ≥ 3.10.1, was der
   CHANGELOG nennt). Sobald §2/§3 geklärt sind, gehört der Wert erneut geprüft.

## Querverweise

- `ctx_knowledge`: `blocker/lean-ctx-addon-system-upstream-entfernt`,
  `decision/lean-md-0-2-3-lokal-von-hand-installiert`,
  `decision/release-0-2-3-vorbereitet-binary-only`
- `CHANGELOG.md` → `[binary 0.2.3]`
- `docs/RELEASING.md`, `docs/dev-readme.md` (beide korrekturbedürftig, siehe §4)
