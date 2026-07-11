# Design-Spec: V2 (Tag `v0.2.0` + echte SHA-256) + Registry-Nachzug §5.8

> Erstellt: 2026-07-11 · Branch: `feat-lmd-v2` · Ansatz **A** (lokale Pre-Flight-Checks
> → direkt `v0.2.0`, kein RC).
>
> **Übergeordnete Spec:** `2026-07-10-lmd-release-path-rev2-design.md`. Diese Spec
> schneidet daraus **eine** Phase heraus: Gate **V2** plus den dort in §5.8 offenen
> Registry-Nachzug. **Nicht** enthalten: V4a (Pack publish), V4b (`addon publish`),
> Smoke Teil 2, Voll-Smoke — spätere Phasen.

---

## 1. Scope & Gates

Diese Phase schließt:

- **V2** — Tag `v0.2.0` gesetzt, GitHub-Release mit fünf Binaries + `SHA256SUMS`
  erzeugt, die fünf `[artifacts.*].sha256` in `lean-ctx-addon.toml` tragen echte
  SHA-256 (kein `0000…`).
- **Registry-Nachzug §5.8** — der curated Entry in `lean-ctx`
  (`rust/data/addon_registry.json`) wird auf denselben `3.9.6`-Vertrag gehoben und
  erhält die Release-Version.

Verifizierter Ausgangsstand (2026-07-11):

- `origin/feat-lmd-v2` existiert und ist der **öffentliche Default-Branch**
  (`origin/HEAD → origin/feat-lmd-v2`) auf `github.com/dasTholo/lean-md`; lokal
  **44 Commits ahead**. Der Push wurde per Push-Politik-Entscheidung sanktioniert
  (die Paket-C-„lokal halten"-Klausel gilt als aufgehoben).
- Keine Tags im Repo (`git tag --list` leer).
- `lean-ctx-addon.toml`: `min_lean_ctx = "3.9.6"` (line 11), fünf `[artifacts.*]`-Blöcke
  (line 20–43) mit `sha256 = "0000…"`; Triples deckungsgleich mit der 5-Leg-Matrix
  in `release.yml`.
- Registry-Entry `lean-ctx/rust/data/addon_registry.json` line 411:
  `min_lean_ctx = "3.9.4"`; line 393: `version = ""`; `verified = false`.

---

## 2. Warum der GitHub-Weg de facto zwingend ist

Die fünf `[artifacts].url` sind **hart** auf
`https://github.com/dasTholo/lean-md/releases/download/v0.2.0/<asset>` verdrahtet.

- **GitHub-Release als Host: zwingend.** Die `sha256` muss zu den Bytes passen, die
  unter genau diesen URLs ausgeliefert werden. Es muss ein `v0.2.0`-Release mit den
  fünf Assets an diesen URLs geben. Anderer Host ⇒ URLs ändern ⇒ Out-of-Scope.
- **GitHub *Actions* als Builder: nicht zwingend, aber praktikabel-alternativlos.**
  Man könnte lokal bauen, per `gh release create` hochladen und den Patch lokal
  fahren — aber die macOS-Legs (`x86_64`/`aarch64-apple-darwin`) baut kein Linux-Host
  (kein osxcross), und `x86_64-pc-windows-msvc` (MSVC-ABI) liefert mingw nicht sauber.
  Actions stellt macOS- und Windows-Runner bereit.

∴ Der **Tag → Actions**-Weg ist der reale Pfad, solange kein Mac + Windows-Rechner
danebensteht. Ein lokales Script (§3) ist deshalb ein **Confidence-Dry-Run**, kein
CI-Ersatz; die einzige Quelle realer SHAs bleibt CIs `sync-manifest` — Single Source
of Truth.

---

## 3. Pre-Flight (lokal, nicht-mutierend)

Zwei Checks vor dem Tag fangen die zwei realistischen Fehlerquellen ab; **keiner**
schreibt in das echte Manifest.

### 3.1 Build-Leg

```
cargo build --release --locked --target x86_64-unknown-linux-gnu
```

Grün ⇒ die Release-Legs kompilieren mit `--locked` (kein `Cargo.lock`-Drift). Fängt
Build-Bruch.

### 3.2 Patch-Regex-Dry-Run

Die exakte `sync-manifest`-Python-Logik aus `release.yml` gegen ein **synthetisches**
`SHA256SUMS` laufen lassen (fünf Asset-Namen; für das lokale Triple die **echte**
sha256 des in 3.1 gebauten Binaries, für die übrigen vier eine Dummy-SHA), Ziel ist
eine **Temp-Kopie** von `lean-ctx-addon.toml`. Erwartung:

- pro `[artifacts.*]`-Block genau ein Treffer (`n == 1`),
- `missing == ∅` (jeder erwartete Triple hat eine `SHA256SUMS`-Zeile),
- `patched != src` (mindestens ein Slot geändert).

Fängt Regex-/Triple-Drift zwischen `release.yml` und dem aktuellen Manifest.

**Optionaler Cross-Leg (kein Muss):** `aarch64-unknown-linux-gnu` (via
`gcc-aarch64-linux-gnu`) liefert eine weitere **echte** lokale SHA für den Dry-Run —
erhöht die Aussagekraft, ändert die Konklusion nicht. `x86_64-pc-windows-msvc` ist
bewusst **nicht** aufgeführt: mingw produziert die MSVC-ABI nicht deckungsgleich (§2),
ein lokaler Windows-Build wäre für den Dry-Run irreführend. macOS bleibt CI-only, das
echte Manifest wird nie lokal geschrieben.

> Umsetzung im Plan: als einmaliges Runbook-Snippet, nicht als eingecheckter
> Workflow-Bestandteil. Es dupliziert bewusst die `release.yml`-Python 1:1, damit der
> Dry-Run dieselbe Regex prüft, die CI später fährt.

---

## 4. Cut V2

Reihenfolge bindend:

1. `git push origin feat-lmd-v2` — 44 Commits. **Kein Trigger** (`release.yml` hängt
   am Tag `v[0-9]*`, nicht am Branch-Push). Bringt u.a. `min_lean_ctx = "3.9.6"` und
   die `[artifacts]`-Blöcke öffentlich in Deckung mit dem gleich getaggten Stand.
2. `git tag v0.2.0 <HEAD>` → `git push origin v0.2.0`.
3. CI (`release.yml`): 5-Leg-Build (`--release --locked`) → GH-Release `v0.2.0`
   (fünf bare Binaries + `SHA256SUMS`) → `sync-manifest` patcht die fünf echten
   SHA-256 in `lean-ctx-addon.toml` und bot-committet auf `feat-lmd-v2`
   (`chore(release): sync [artifacts] sha256 for v0.2.0`).

**Bewusst — Tag trägt weiter `0000…`:** `sync-manifest` patcht den **Branch**, nicht
rückwirkend den Tag. Der `v0.2.0`-Tag zeigt im Manifest dauerhaft `0000…`; die echten
SHAs leben auf `feat-lmd-v2` und feeden das spätere V4b (`addon publish`). Das ist
korrekt: Consumer ziehen den publizierten Addon (V4b, andere Phase), nicht den Tag.

---

## 5. Verifikation (DoD V2)

Nach CI-Durchlauf:

- `git pull origin feat-lmd-v2` — holt den `sync-manifest`-Bot-Commit.
- `lean-ctx-addon.toml`: fünf `[artifacts.*].sha256` sind echte SHA-256, kein `0000…`.
- GH-Release `v0.2.0`: fünf Binaries + `SHA256SUMS` vorhanden.
- **Kreuzprobe:** die fünf Manifest-SHAs matchen die Zeilen in `SHA256SUMS`
  (`sync-manifest` schreibt exakt diese; die Kreuzprobe fängt einen fehlgeschlagenen
  Patch-Teil ab).

---

## 6. Registry-Nachzug §5.8 (Upstream `lean-ctx`)

**Vorbedingung: V2 erfolgreich** (Kopplung an `version = "0.2.0"`).

In `/home/tholo/Scripts/lean-ctx/rust/data/addon_registry.json`, Entry `lean-md`:

- line 411: `min_lean_ctx "3.9.4" → "3.9.6"` (nach D3 der Rev2-Spec — der Wert muss
  denselben Vertrag behaupten wie lean-mds Manifest).
- line 393: `version "" → "0.2.0"` (stempelt die Release-Version; spart einen zweiten
  Registry-PR).
- `verified = false` **bleibt** — registry-/Upstream-Hoheit, in einem eingereichten
  Entry bedeutungslos.

Lieferung: eigener PR gegen `yvgude/lean-ctx`. Unabhängig von der lean-md-CI; keine
lean-md-Code-Abhängigkeit.

---

## 7. Error-Handling & Rollback

| Fall | Verhalten | Reaktion |
|---|---|---|
| Leg-Bruch in CI | `build`-Job rot ⇒ `release` (needs:build) läuft nicht ⇒ keine SHAs, Tag „leer" | Fix, dann **`v0.2.1`** (Tags **nicht** force-retaggen — Release-Assets/Tag immutable behandeln) |
| `sync-manifest` Push-Konflikt | Bot-Push scheitert an vorgelaufenem `feat-lmd-v2` | lokal `git pull` vor weiteren Commits; ggf. Job re-run |
| Patch-Teil trifft nicht (`n != 1`) | Python `raise SystemExit`, Job rot, Manifest unverändert | Pre-Flight §3.2 macht das unwahrscheinlich; Regex/Triple-Drift lokal beheben, neuer Tag |
| macOS-Leg flaky | einzelnes Leg rot ⇒ ganzer Release blockiert | Re-run des Legs; kein neuer Tag nötig, solange der Tag steht und `release` noch nicht lief |

---

## 8. Definition of Done

- **V2:** Tag `v0.2.0` gesetzt; GH-Release mit 5 Binaries + `SHA256SUMS`;
  `lean-ctx-addon.toml` auf `feat-lmd-v2` trägt fünf echte SHA-256, matchend zu
  `SHA256SUMS`; kein `0000…` mehr.
- **§5.8:** Registry-PR gegen `yvgude/lean-ctx` offen/gemergt mit
  `min_lean_ctx = "3.9.6"` und `version = "0.2.0"` im `lean-md`-Entry.

**Nicht enthalten:** V4a/V4b, Smoke Teil 2, Voll-Smoke; keine `release.yml`-Änderung
(der RC-Weg — und damit der `sync-manifest`-Prerelease-Guard — ist entfallen).
