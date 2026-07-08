# Design: lean-md P0 — crates.io-Publish-ready + `[install] manager=cargo`

> # ⛔ ÜBERHOLT (2026-07-08) — crates.io verworfen
>
> Dieses Design (crates.io-Publish + `[install] manager=cargo`) ist **nicht mehr** der
> Umsetzungsweg. lean-md wird nur über `addon add` konsumiert → der Binary-Kanal ist der
> **`[artifacts]`-Block** (prebuilt Binaries von GitHub-Release, lean-ctx lädt + verifiziert
> SHA-256). Maßgeblich sind:
> - **Plan:** `docs/lean-md/plans/2026-07-08-lmd-dist-p0-artifacts-release-plan.lmd.md`
> - **Kontext/Roadmap:** `lean-md-next-session.prompt.md` (2026-07-08-Pivot)
>
> Dieses Doc bleibt als **Entscheidungshistorie** erhalten. Übertrag der Review-Befunde:
> - **B3** (Registry-Entry drüben live + pinnt 0.2.0 → `addon add` rot bis Release) **gilt weiter**.
> - **B1** (`[install]`-Schema) → **ersetzt** durch das `[artifacts]`-Schema (filename/url/sha256).
> - **B2** (`[[example]]`-Publish-Verify) → **gegenstandslos** (kein `cargo publish` mehr).
> - lean-ctx-Gegenstück-Branch ist **`pr-rebuild`** (nicht `feat-lmd-v2`, wie unten noch steht).

- **Datum:** 2026-07-07
- **Branch:** `feat-lmd-v2` (Gegenstück in lean-ctx: gleichnamig)
- **Scope:** **nur P0** (Binary-Kanal). P1–P3 sind separate Sessions — als Follow-ups
  in der lean-ctx-Knowledge festgehalten (`lean-md-dist-follow-up-p1/p2/p3`), nicht hier.
- **Quellen (verifiziert):** PR #721 (3 Maintainer-Kommentare), EPIC #724,
  Issues #725/#726/#727, `docs/specs/unified-distribution-v1.md` (lean-ctx-Repo).

> **Review 2026-07-08 (Design-Überprüfung, lean-ctx-Seite erfüllt):** Vier
> Korrekturen gegen den Ist-Zustand eingefolded:
> - **B1** Pre-Flight ergänzt — `docs/CONTRACT.md` ist auf `2946c165a` (prä-Rails)
>   gepinnt und dokumentiert **keinen** `[install]`-Block → re-vendorn + Feldnamen
>   verifizieren, bevor das Manifest editiert wird (siehe „0. Pre-Flight").
> - **B2** `[[example]]` aus dem Publish genommen (blockierte sonst den
>   dry-run-Verify-Build) — siehe Änderung 4.
> - **B3** „Sync-Vertrag"-Absatz korrigiert: der Registry-Entry drüben ist bereits
>   **live und pinnt 0.2.0** → bewusstes Dangling-Fenster bis Post-P3-Publish
>   (nicht „vorbereitet, nicht live").
> - **B3-Staleness** Risiko #2 (3.9.1) obsolet — installierter `lean-ctx` ist
>   jetzt **3.9.2**.
> Bestätigt: `lean-ctx-client 0.1.0` liegt real auf crates.io; `README.md`
> existiert (readme-Pin safe); Capability-Enum ist `none|full` → `network="full"`
> korrekt.

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

**Folgen für den Sync-Vertrag (korrigiert 2026-07-08 — B3):** der lean-ctx-
`feat-lmd-v2`-Registry-Entry ist bereits **live** und pinnt `0.2.0`. Da `lean-md`
noch **nicht** auf crates.io liegt (verifiziert: NoSuchKey), ist das ein **bewusst
in Kauf genommenes Dangling-Fenster**: `lean-ctx addon add lean-md` liefert bis
zum Post-P3-`cargo publish` einen Fetch-Fehler (crates.io kennt `lean-md` noch
nicht). Das ist ein **bekannter, dokumentierter Zustand** — nicht „vorbereitet,
nicht live". `0.2.0` bleibt der Sync-Anker (crates.io == `[install].version`); der
Maintainer (@dasTholo) schließt das Fenster mit dem einmaligen Post-P3-Publish.
∴ P0 macht den Binary-Kanal **publish-ready**, aber der end-to-end-`addon add`
bleibt bis dahin erwartungsgemäß rot — das ist keine Regression, sondern der
vereinbarte Zwischenzustand.

## Abgrenzung — was diese Session NICHT tut

- **Kein echter `cargo publish`** (siehe Publish-Strategie oben — bis nach P3
  aufgeschoben).
- **Kein Edit am lean-ctx-Registry-Entry.** `addon_registry.json` ist seit PR #734
  **generiert** (`gen_registry` + Drift-Check-CI) → kein Handedit. Der Registry-
  Eintrag drüben ist ein **Handoff** (Snippet unten), Arbeit im lean-ctx-Repo.
- **Kein Skills-Ausbau (P3).** Skills bleiben `include_str!`-embedded.

## Änderungen

### 0. Pre-Flight (B1) — `docs/CONTRACT.md` re-vendorn, `[install]`-Schema verifizieren

**Blocker vor jedem Manifest-Edit.** Das vendored `docs/CONTRACT.md` ist auf
lean-ctx `2946c165a` gepinnt (prä-Distribution-Rails) und dokumentiert **nur**
`[addon]`/`[mcp]`/`[capabilities]` — **keinen `[install]`-Block**. Die Feldnamen
`manager/package/version/bin` stammen aus der lean-ctx-Repo-Spec (PR #721), **nicht**
aus dem vendored Kontrakt. Projekt-Regel (`AGENTS.md`): „Never reconstruct schemas
from memory; CONTRACT.md ist die Quelle."

**Verifiziert 2026-07-08:** Die `[install]`-Schema-Quelle ist **nicht** die
Manifest-Contract-Doc (die kennt den Block nicht), sondern
`docs/dev/addon-bootstrap-engine.md` im lean-ctx-Repo. Die Feldnamen des Designs
**stimmen exakt** (bestätigt gegen die Quelle):

```toml
[install]
manager = "cargo"   # Enum: pip|uv|cargo|npm|brew|dotnet
package = "lean-md"
version = "0.2.0"   # MANDATORY exakter Pin (keine Ranges/latest), sonst validate()-Reject
bin     = "lean-md"
# verify = [...]     # optional; ohne verify prüft die Engine `bin` auf PATH
```

Enforced by `AddonInstall::validate()`; `cargo`-argv = `cargo install {base}
--version {version}` → `cargo install lean-md --version 0.2.0` (deckt sich mit der
out-of-scope-Verifikationszeile). `package/version/bin` dürfen keine
Shell-Metazeichen enthalten — `lean-md` ist sauber.

Schritte (Task 1 im Plan):
1. Den `[install]`-Schema-Block als **lokales vendored Addendum** in `docs/CONTRACT.md`
   festhalten (Quelle + lean-ctx-Commit zitieren) — die Manifest-Contract-Doc allein
   deckt ihn nicht ab, künftige Arbeit soll ihn lokal nachschlagen können.
2. Feldnamen bestätigt (s.o.) → Änderung 2 (`lean-ctx-addon.toml`) mit exakt diesen
   Feldern schreiben.

∴ B1 ist damit von „unbekannt/Blocker" auf „bestätigt, nur lokal vendoren"
zurückgestuft — kein Feldnamen-Drift.

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

> **Status 2026-07-08:** drüben **erledigt** — der Entry ist live und pinnt `0.2.0`
> (B3). Dieser Abschnitt ist damit retrospektiv der erfüllte Handoff, nicht mehr
> offene Arbeit. Die Konsequenz (Dangling bis Publish) steht in der Publish-Strategie.

### 4. `Cargo.toml` — `[[example]]` aus dem Publish nehmen (B2)

```diff
-[[example]]
-name = "skill-token-comparison"
-path = "benchmarks/skill-token-comparison/main.rs"
```

**Begründung:** `cargo publish --dry-run` fährt standardmäßig einen Verify-Build,
der **alle deklarierten Targets** (inkl. Examples) kompiliert. `benchmarks/**` ist
**nicht** in `include` → die Example-Quelle fehlt im gepackten Crate → der
Verify-Build scheitert. Der Token-Vergleich ist ein **Bench-Harness**, gehört nicht
ins veröffentlichte Binary-Crate.

- Der Stanza-Entfall macht `benchmarks/skill-token-comparison/main.rs` zu einem
  **nicht-deklarierten** Cargo-Target — die Datei bleibt on-disk (git-tracked),
  wird aber beim Publish-Verify nicht mehr gebaut.
- `tiktoken-rs` bleibt `dev-dependency` (nur der Harness braucht es; keine Wirkung
  aufs Publish-Paket).
- Alternative (verworfen): `benchmarks/**` in `include` — würde den Harness ins
  veröffentlichte Crate ziehen; unnötiger Ballast für ein CLI-Binary.
- Falls der Harness weiter per `cargo run --example` laufbar sein soll, später als
  **nicht-publizierter Workspace-Member** wieder einhängen (out-of-scope P0).

## Install-UX (Endzustand nach P3) — ein Befehl, zwei Kanäle

**Forward-looking, nicht P0-Arbeit** — aber es rahmt, wofür die `[install]`-Config
gebaut wird. Der User tippt **einen** Befehl (Verb ist `addon add`, nicht
`install`):

```
lean-ctx addon add lean-md
   │  depth-1 Dependency-Resolution (#743, gemergt)
   ├─ @dasTholo/lean-md        (kind=addon)   → Binary   (crates.io / [artifacts])
   └─ @dasTholo/lean-md-skills (kind=skills)  → Skills-Pack   ← als Dependency deklariert
        ein Consent-Prompt listet BEIDES · ctxpkg.lock hält das aufgelöste Paar
```

Der Skills-Pack ist **kein zweiter Install-Schritt**, sondern eine deklarierte
Dependency im `lean-ctx-addon.toml` (kommt **erst in P3**):

```toml
[[dependencies]]
name = "@dasTholo/lean-md-skills"
version = "^1.0"
```

| Aktion | Befehl | Anmerkung |
|---|---|---|
| Addon **+** Skills | `lean-ctx addon add lean-md` | ein Prompt, beide landen; Lockfile hält das Paar |
| Update (inkl. Skills) | `lean-ctx addon update lean-md` | refreshed die Skills-Dependency auch bei aktuellem Binary → Skill-Fix ohne Binary-Release |
| (optional) nur der Pack | `lean-ctx pack install @dasTholo/lean-md-skills` | Sonderfall; Normalweg ist `addon add` |

**Zwei getrennte Kanäle** (Binary via crates.io/`[artifacts]`, Skills via
ctxpkg-Pack) mit **unabhängigen Kadenzen**, hinter **einem** Befehl vereint —
genau die „ein Consent-Prompt, zwei Kadenzen"-Mechanik aus EPIC #727. **In P0
existiert die `[[dependencies]]`-Zeile noch nicht** (Skills bleiben embedded); P0
legt nur den `[install]`-Block (Binary-Kanal), auf dem P3 aufsetzt.

## Verifikation

| Schritt | In dieser Session | Wer |
|---|---|---|
| `cargo build` (default) + `cargo build --features mcp` grün | ✅ | Plan |
| `cargo fmt` + `cargo nextest run` grün (keine Regression) | ✅ | Plan |
| `docs/CONTRACT.md` re-vendort (3.9.2) + `[install]`-Feldnamen verifiziert (B1) | ✅ | Plan |
| `[[example]]` aus `Cargo.toml` entfernt → Publish-Verify baut keinen Bench (B2) | ✅ | Plan |
| `cargo publish --dry-run` sauber (keine path-only deps, Metadaten valide, kein Example-Verify-Fail) | ✅ | Plan |
| Fragment-Consistency-Gate + Determinismus-Suite (#498) grün | ✅ | Plan |
| `cargo publish` (echt) | ⛔ out-of-scope | Maintainer |
| `cargo install lean-md --version 0.2.0` → `lean-md mcp` antwortet | ⛔ out-of-scope | Maintainer |
| `addon add lean-md` end-to-end | ⚠️ P1+-Thema; PATH-`lean-ctx` ist **3.9.2** (kein Reinstall mehr nötig). Bleibt bis Post-P3-Publish erwartungsgemäß rot (Dangling-Fenster, B3) | Follow-up |

**Determinismus (#498):** reine Metadaten-/Manifest-Edits, kein Render-Output
betroffen — die Determinismus- und Fragment-Consistency-Gates müssen unverändert
grün bleiben.

## Risiken

- **`cargo publish --dry-run` deckt nicht alles ab:** ein path+version-dep kann im
  Dry-Run grün sein, aber der reale Publish scheitert, wenn die crates.io-Version
  API-inkompatibel zur lokalen `path`-Version ist. Mitigation: `mcp`-Feature gegen
  die **crates.io**-`lean-ctx-client 0.1.0` bauen (path temporär entfernen und
  `cargo build --features mcp` prüfen), bevor der Maintainer publisht.
- **~~Installierter Binary meldet 3.9.1~~ — aufgelöst (2026-07-08):** der auf PATH
  laufende `lean-ctx` ist **jetzt 3.9.2** (`lean-ctx --version` verifiziert), also
  identisch zu Source/CHANGELOG. Die Distribution-Rails (#725/#726) sind live; der
  frühere Reinstall-Vorbehalt für P1+-Live-Tests entfällt. **P0 braucht ohnehin
  kein `addon add`** (nur cargo) — der Punkt ist damit gegenstandslos.
- **`[install]`-Schema-Drift (neu, B1):** der lokale vendored Kontrakt kennt den
  Block nicht (siehe „0. Pre-Flight") → Feldnamen erst nach Re-Vendorn fixieren.

## Definition of Done (diese Session)

- **Pre-Flight (B1):** `docs/CONTRACT.md` aus 3.9.2 re-vendort, `[install]`-Feldnamen
  gegen die frische Quelle verifiziert **bevor** das Manifest editiert wird.
- `Cargo.toml` (inkl. `[[example]]`-Entfall, B2) + `lean-ctx-addon.toml` wie oben;
  `cargo publish --dry-run` sauber (kein Example-Verify-Fail);
  `cargo build --features mcp` gegen crates.io-`lean-ctx-client` grün; Tests + fmt
  + Determinismus-Gates grün; Registry-Handoff-Snippet im Spec dokumentiert.
- **Sync-Realität (B3) dokumentiert:** Registry-Entry drüben live + pinnt 0.2.0 →
  `addon add` bis Post-P3-Publish erwartungsgemäß rot (kein Bug).
- **Nicht** enthalten: scharfer Publish, Live-`addon add`, P1–P3.
