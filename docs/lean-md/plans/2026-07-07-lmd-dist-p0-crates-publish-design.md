# Design: lean-md P0 — crates.io-Publish-ready + `[install] manager=cargo`

- **Datum:** 2026-07-07
- **Branch:** `feat-lmd-v2` (Gegenstück in lean-ctx: gleichnamig)
- **Scope:** **nur P0** (Binary-Kanal). P1–P3 sind separate Sessions — als Follow-ups
  in der lean-ctx-Knowledge festgehalten (`lean-md-dist-follow-up-p1/p2/p3`), nicht hier.
- **Quellen (verifiziert):** PR #721 (3 Maintainer-Kommentare), EPIC #724,
  Issues #725/#726/#727, `docs/specs/unified-distribution-v1.md` (lean-ctx-Repo).

## Ziel

`cargo install lean-md` funktioniert, und der lean-ctx-Registry-Entry mit
`[install] manager=cargo, version=0.2.0` macht `addon add lean-md` grün — **ohne
Side-Loading**. Der **Version-Pin `0.2.0` ist der Cross-Repo-Vertrag**: crates.io-
`version` == `[install].version` im Registry-Entry drüben.

**Reverse-Cut bleibt intakt:** kein Engine-Symbol in `lean-ctx/rust/src`; lean-md
bleibt standalone (`rushdown` + `evalexpr`), Code-Intel outbound über `ctx_*`.

## Publish-Strategie — scharfer Publish aufgeschoben bis nach P3

**Entscheidung (2026-07-07):** Der echte `cargo publish` wird **bis nach P3
aufgeschoben**. Bis dahin ist `cargo publish --dry-run` das **stehende
Verifikations-Gate** — es läuft in P0 (und bleibt in P1–P3 grün), aber **nichts
geht nach crates.io**, bevor der Skills-Pack (P3) steht.

Begründung: crates.io ist append-only (Publish irreversibel, nur `yank`). Ein
einziger scharfer Publish am Ende vermeidet verbrauchte Zwischenversionen. **Die
Version bleibt durchgehend `0.2.0`** — da der erste und einzige Publish nach P3
erfolgt, enthält `0.2.0` bereits P0–P3 (inkl. Skills-Pack); es gibt keinen
`0.3.0`-Zwischenschritt. Der finale Publish + die `cargo install`-Verifikation
macht der Maintainer (@dasTholo) selbst.

**Folgen für den Sync-Vertrag:** solange nicht publiziert ist, kann der lean-ctx-
`feat-lmd-v2`-Registry-Entry auf **keine** echte Version zeigen → er bleibt
ebenfalls „vorbereitet, nicht live". `0.2.0` ist und bleibt der Sync-Anker
(crates.io == `[install].version`).

## Abgrenzung — was diese Session NICHT tut

- **Kein echter `cargo publish`** (siehe Publish-Strategie oben — bis nach P3
  aufgeschoben).
- **Kein Edit am lean-ctx-Registry-Entry.** `addon_registry.json` ist seit PR #734
  **generiert** (`gen_registry` + Drift-Check-CI) → kein Handedit. Der Registry-
  Eintrag drüben ist ein **Handoff** (Snippet unten), Arbeit im lean-ctx-Repo.
- **Kein Skills-Ausbau (P3).** Skills bleiben `include_str!`-embedded.

## Änderungen

### 1. `Cargo.toml` — publish-ready

```diff
-version = "0.1.0"
+version = "0.2.0"
+repository = "https://github.com/dasTholo/lean-md"
+readme = "README.md"
+keywords = ["markdown", "macros", "llm", "context", "skills"]   # ≤5, je ≤20 Zeichen
+categories = ["command-line-utilities", "text-processing"]       # gültige crates.io-Slugs

 [dependencies]
-lean-ctx-client = { path = "…/lean-ctx-client", optional = true }
+lean-ctx-client = { version = "0.1", path = "…/lean-ctx-client", optional = true }

 [features]
 default = []      # unverändert: CLI-only im Standard-Binary
 mcp = ["dep:lean-ctx-client"]
```

**Begründung:**
- `version="0.1"` **neben** `path`: lokal gewinnt der Pfad (Dev gegen lokales
  lean-ctx), beim Publish nutzt cargo die crates.io-Version (`lean-ctx-client 0.1.0`
  ist bereits publiziert) → **Publish-Blocker gelöst**. Der Fix ist **unabhängig
  vom Feature-Default**: cargo verlangt die Version nur zur Validierung der
  optionalen dep.
- `default = []` **bleibt** — CLI-first-Haltung (`AGENTS.md`, null-`lean_ctx`-
  Invariante). Das ausgelieferte `cargo install`-Binary ist CLI-only und schlank;
  MCP-Backend opt-in via `cargo install lean-md --features mcp`. `CliBackend` ist
  und bleibt der Default-*Backend* zur Laufzeit.
- `keywords`/`categories` müssen crates.io-valide sein (`categories` gegen die
  feste Slug-Liste; `keywords` max. 5, je ≤20 Zeichen). Beim `--dry-run` prüfen.

### 2. `lean-ctx-addon.toml` — Authoring-Manifest

```diff
-version = "0.1.0"
+version = "0.2.0"
-min_lean_ctx = "3.8.12"
+min_lean_ctx = "3.9.2"

+[install]
+manager = "cargo"
+package = "lean-md"
+version = "0.2.0"        # == crates.io-Pin (Sync-Vertrag)
+bin     = "lean-md"

 [capabilities]
-network = "none"
+network = "full"         # honest: cargo-Fetch zur Install-Zeit (Maintainer-Vorgabe, PR #721 Kommentar 1)
 filesystem = "read_write"
 exec = ["lean-ctx"]
```

**Begründung:**
- `[install]`-Block ist der Kern von P0 — er sagt der lean-ctx-Bootstrap-Engine, wie
  sie das Binary provisioniert (pinned, shell-free).
- `network=full`: der Maintainer verlangt es explizit als ehrliche Capability für
  den Package-Fetch. Runtime bleibt CLI-only (kein Socket) — die Capability deckt
  den Install-Fetch ab.
- `min_lean_ctx=3.9.2`: Version, in der die Distribution-Rails (#725/#726) landeten.

### 3. Registry-Entry drüben (`feat-lmd-v2`) — HANDOFF, nicht hier gebaut

Im lean-ctx-Repo über die `gen_registry`-Snapshot-Quelle einzupflegen (kein
Handedit an `addon_registry.json`; `gen_registry --check` ist CI-Gate):

```toml
[install]
manager = "cargo"
package = "lean-md"
version = "0.2.0"   # == crates.io-Version oben
bin     = "lean-md"
```

Plus `min_lean_ctx = "3.9.2"` und `network = "full"` im Entry spiegeln.

## Verifikation

| Schritt | In dieser Session | Wer |
|---|---|---|
| `cargo build` (default) + `cargo build --features mcp` grün | ✅ | Plan |
| `cargo fmt` + `cargo nextest run` grün (keine Regression) | ✅ | Plan |
| `cargo publish --dry-run` sauber (keine path-only deps, Metadaten valide) | ✅ | Plan |
| Fragment-Consistency-Gate + Determinismus-Suite (#498) grün | ✅ | Plan |
| `cargo publish` (echt) | ⛔ out-of-scope | Maintainer |
| `cargo install lean-md --version 0.2.0` → `lean-md mcp` antwortet | ⛔ out-of-scope | Maintainer |
| `addon add lean-md` end-to-end | ⚠️ P1+-Thema; braucht **3.9.2**-Binary (Source ist 3.9.2, PATH-Binary meldet noch 3.9.1 → reinstall) | Follow-up |

**Determinismus (#498):** reine Metadaten-/Manifest-Edits, kein Render-Output
betroffen — die Determinismus- und Fragment-Consistency-Gates müssen unverändert
grün bleiben.

## Risiken

- **`cargo publish --dry-run` deckt nicht alles ab:** ein path+version-dep kann im
  Dry-Run grün sein, aber der reale Publish scheitert, wenn die crates.io-Version
  API-inkompatibel zur lokalen `path`-Version ist. Mitigation: `mcp`-Feature gegen
  die **crates.io**-`lean-ctx-client 0.1.0` bauen (path temporär entfernen und
  `cargo build --features mcp` prüfen), bevor der Maintainer publisht.
- **Installierter Binary meldet 3.9.1, Source/CHANGELOG ist 3.9.2:** die
  Distribution-Rails (#725/#726) sind in 3.9.2, aber der auf PATH laufende
  `lean-ctx` ist noch 3.9.1 → für P1+-Live-Tests (`addon publish`, `[artifacts]`)
  muss die 3.9.2-Build neu installiert werden. **P0 braucht kein `addon add`** —
  nur cargo — und ist davon unberührt.

## Definition of Done (diese Session)

- `Cargo.toml` + `lean-ctx-addon.toml` wie oben; `cargo publish --dry-run` sauber;
  `cargo build --features mcp` gegen crates.io-`lean-ctx-client` grün; Tests + fmt
  + Determinismus-Gates grün; Registry-Handoff-Snippet im Spec dokumentiert.
- **Nicht** enthalten: scharfer Publish, Live-`addon add`, P1–P3.
