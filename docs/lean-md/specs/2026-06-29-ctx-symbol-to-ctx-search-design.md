# Spec: `ctx_symbol` → `ctx_search` Migration + neue `@symbol body` Op

**Datum:** 2026-06-29
**Branch:** feat-lmd-v2
**Status:** Design (genehmigt) → bereit für Implementierungsplan

## Kontext

lean-ctx main hat das Tool `ctx_symbol` deprecated; der Symbol-Body-Lookup
(„AST-präziser Body eines Symbols by name") läuft jetzt über
`ctx_search action="symbol" name=…`.

**Befund aus dem lean-md-Code (verifiziert):** lean-md ruft `ctx_symbol`
**zur Laufzeit nie auf**. Über alle 36 `backend.call(...)`-Stellen im `src`
hinweg existiert **kein** `call("ctx_symbol", …)`.

- Die `@symbol`-Bridge (`src/bridges/symbol.rs`) routet nav-ops
  (`refs/def/impl/declaration/type-hierarchy`) zu **`ctx_refactor`** und
  `overview` zu `ctx_refactor symbols_overview` bzw. lokalem CRP-tree-sitter
  (`symbol.rs:108`).
- `@search` (`src/bridges/search.rs`) routet zu **`ctx_search`** (Felder
  `pattern/path/ext/max_results` — kein `action`).
- `ctx_symbol` lebt nur als **benanntes Tool im Doku-Cheat-Sheet**: Seeds,
  Living-Docs und ein String-`contains`-Test.

∴ Die Deprecation trifft **keinen aktiven Codepfad**. Die Bridge ist gegen die
Entfernung von `ctx_symbol` immun, weil sie das Tool nie benutzt hat. Was
wirklich an der Bridge hängt, ist `ctx_refactor` (muss erhalten bleiben).

**Latente Funktionschance:** `@symbol` lehnt heute `name=` ohne `line=` mit
einem ERROR ab (`resolve_name_path` ist outbound nicht verfügbar). Das neue
`ctx_search action="symbol" name=X` stellt genau diesen Lookup wieder her.

## Scope (genehmigt)

**Doku-/Seed-Hygiene + neue funktionale Op.** Konkret:

1. Doku/Seed/Test: alle `ctx_symbol`-Referenzen → `ctx_search action=symbol`.
2. Neue Bridge-Op `body`: `@symbol body name=X` → `ctx_search action=symbol`.

**Nicht im Scope:** nav-ops (`refs/def/impl/…`) bleiben auf `ctx_refactor`
(`ctx_search` kann keine LSP-Nav — Locations/Caller/Hierarchie). Historische
plan/spec-Docs bleiben als datierte Aufzeichnungen unverändert.

## Design

### 1. Neue Bridge-Op `body` (`src/bridges/symbol.rs`)

- **Syntax:** `@symbol body name=X [file=…] [kind=fn|struct|class|trait|enum] [path=…]`
- **Routing:** `ctx.backend.call("ctx_search", { action:"symbol", name, file?, kind?, path? })`.
- **Pflichtarg:** `name=` → sonst `BridgeError::MissingArg("name")`.
- **Dispatch:** Sonderfall vor `nav()`, analog zu `overview` — in
  `SymbolBridge::execute` nach `map_op`-Auflösung. `body` wird **nicht** über
  `map_op` auf eine `ctx_refactor`-Action gemappt, sondern eigener Zweig.
- **CRP:** Output **verbatim** durchreichen (kein lokales CRP-Signaturrendering).
  Begründung: der Body ist bereits AST-präzise; CRP-Signaturrendering ist für
  Overviews gedacht, nicht für Bodies. Konsistent mit den nav-ops.
- **Jail:** `path`/`file`, sofern als Pfad gegeben, über
  `crate::pathx::resolve_tool_path` auflösen (wie nav/overview). `name`/`kind`
  unverändert durchreichen.

### 2. ERROR-Pfad anpassen (gleiche Datei, `nav()`)

- Bestehende `name=`-ohne-`line=`-Meldung zeigt neu auf die echte Lösung:

  > `ERROR: name= addressing needs line= for nav ops; for a symbol body use '@symbol body name=…'`

- unknown-op-Meldung um `body` ergänzen:
  `Use: refs|def|impl|declaration|type-hierarchy|overview|body`.

### 3. Seeds (byte-stabil, #498)

- `content/core/hard-rules.lmd.md` (Z. 4):
  `ctx_refactor / ctx_symbol (@symbol)` → `ctx_refactor / ctx_search:symbol (@symbol)`.
- `content/core/_fragments/tool-quick-ref.lmd.md` (Z. 3):
  `@symbol=ctx_refactor/ctx_symbol` → `@symbol=ctx_refactor/ctx_search:symbol`.
- `include_str!`-Konsistenz bleibt automatisch grün (die Built-in-Const liest
  dieselbe Datei zur Compile-Zeit; das byte-identisch-Gate
  `builtin_fragments_match_seed_files_on_disk` bleibt erfüllt).

### 4. Test-Flip (`src/fragments.rs:157-160`)

- Assertion `out.contains("ctx_symbol")` → `out.contains("ctx_search")`;
  Fehlermeldung entsprechend (`"hard-rules must name ctx_search for *.rs"`).
- byte-identisch-Gate (`:164`) bleibt unverändert grün.

### 5. Living-Docs

- `AGENTS.md:16`, `docs/21-lean-md.md:123`, `docs/appendix-lean-md.md:20`:
  `ctx_symbol` → `ctx_search action=symbol`.
- **Unverändert:** `docs/lean-md/plans/…`, `docs/lean-md/specs/…` (datierte
  Aufzeichnungen).

## Tests (neu, in `symbol.rs`)

| Test | Erwartung |
|------|-----------|
| `body_op_is_registered_and_routes` | `@symbol body name=X` dispatcht (kein unknown-op); liefert live-hit \| `BACKEND_REQUIRED` \| `Err(Backend)` — nie Panik |
| `body_missing_name_errors` | `BridgeError::MissingArg("name")` |
| `body_forwards_file_and_kind` | Payload enthält `file`/`kind` wenn gesetzt (via RecordingBackend, falls vorhanden; sonst Dispatch-Assertion) |
| `op_aliases_map_to_ctx_refactor_actions` | um `body` erweitern (bzw. expliziter `body`-Zweig-Test, da nicht über `map_op`) |
| `unknown_op_is_a_clear_error` | unverändert — `body` ist jetzt bekannt |
| `name_addressing_without_line_returns_clear_error` | Meldung enthält neu `@symbol body` |

## Verifikation

- `cargo nextest run` (nie `cargo test`) — alle grün.
- Zero clippy warnings.
- `cargo fmt` pro geänderter Datei vor `git add`.
- Determinismus (#498): Seed-Edits byte-stabil; keine Timestamps/Counter im Output.

## Risiken / Nicht-Ziele

- **Risiko:** `ctx_search action="symbol"` muss in der gepinnten lean-ctx-Version
  (vgl. `docs/CONTRACT.md`) verfügbar sein. Vor Implementierung gegen den
  Contract / `appendix-mcp-tools.md` prüfen.
- **Nicht-Ziel:** nav-ops migrieren; `name=`-Auto-Routing ohne explizite
  `body`-Op (verworfen — vermischt Body-Fetch mit nav-Semantik).
