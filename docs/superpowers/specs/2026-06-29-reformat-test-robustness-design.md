# Design: Reformat-Test-Robustheit (rustfmt-Fallback-Entkopplung)

**Datum:** 2026-06-29
**Repo:** lean-md
**Status:** Design genehmigt → bereit für Implementierungsplan
**Bezug:** `TODO.md` — „2 reformat-Tests degradieren nicht headless, wenn `rustfmt` im PATH ist"

## Problem

Zwei Tests schlagen bei `cargo nextest run` fehl, sobald `rustfmt` im PATH ist
(d. h. auf jeder Dev-Maschine mit Rust-Toolchain):

- `bridges::reformat::tests::returns_backend_required_envelope_headless` (`src/bridges/reformat.rs:114`)
- `engine::tests::reformat_renders_backend_required_e2e` (`src/engine.rs:528`)

### Empirischer Befund (verifiziert 2026-06-29)

- `EngineContext::new` konstruiert über `default_backend` ein `CliBackend`, das zum
  **real installierten** `lean-ctx`-Binary ausshellt (`lean-ctx call ctx_refactor …`).
- Das installierte `lean-ctx` 3.8.15 besitzt im `ctx_refactor reformat`-Pfad einen
  **lokalen rustfmt-Fallback** für `.rs`-Dateien. Verifizierter Output:

  ```
  reformat: '<abs_path>' via rustfmt — changed
  ```

  → **kein** `BACKEND_REQUIRED`, **kein** `ERROR`.
- Der Source-Checkout unter `/home/tholo/Scripts/lean-ctx` degradiert dagegen zu
  `BACKEND_REQUIRED` (`live_jetbrains_backend` → `Err`). Das **installierte Binary
  weicht also vom lokalen Checkout ab** — die Tests dürfen sich nicht auf das
  Degradierungsverhalten einer bestimmten lean-ctx-Version verlassen.

### Ursache (Kernanalyse)

Beide Tests prüfen ein **fremdes** Verhalten („degradiert lean-ctx headless sauber
zu `BACKEND_REQUIRED`?"). Dieses Ergebnis hängt von drei externen Faktoren ab:
lean-ctx-Version, `rustfmt` im PATH und laufende JetBrains-IDE. Reformat ist
produktiv ein **Erfolgs**-Pfad — entweder über die laufende IDE (JetBrains-Plugin)
oder über den lokalen rustfmt-Fallback. Eine fixe `BACKEND_REQUIRED`-Annahme in
lean-md ist damit grundsätzlich falsch.

## Entscheidung

Die lean-md-Tests prüfen ausschließlich **lean-md-Eigenleistung**: dass der
`@reformat`-Bridge korrekt **dispatcht** und den Backend-Output **verbatim
durchreicht**. Erreicht durch ein injiziertes Stub-Backend via
`EngineContext::with_backend` — deterministisch, ohne Abhängigkeit von
lean-ctx-Version, `rustfmt` oder IDE.

„Degradiert lean-ctx headless sauber?" ist **lean-ctx-Vertrag** und gehört in
lean-ctx-Tests, nicht in lean-md.

### Begründung / Konsistenz

- Das `with_backend`-Seam existiert bereits (`src/engine.rs:77`) und ist exakt für
  diesen Zweck dokumentiert („tests / MCP `ctx_md_*` path").
- Etabliertes Mock-Idiom im Repo: `RecordingBackend`, `FailingBackend`,
  `SearchBackend` (inline in `src/phases.rs` Test-Modul).
- Die Wire-Integration `CliBackend` ↔ installiertes lean-ctx ist **bereits separat**
  abgedeckt: `src/backend.rs::cli_backend_calls_ctx_tree`,
  `cli_backend_unknown_tool_errs_or_envelopes` sowie `tests/addon_roundtrip.rs`.
  Reines Mocking dieser zwei Tests erzeugt also keine Abdeckungslücke.

## Änderungen

### 1. `src/bridges/reformat.rs` — `returns_backend_required_envelope_headless`

- **Umbenennen** → `dispatches_reformat_and_passes_output_through`
  (der alte Name behauptet einen Vertrag, der nicht mehr gilt).
- Inline-`StubBackend` (analog `RecordingBackend`): zeichnet `(tool, args)` auf,
  liefert ein **neutrales Sentinel** `"STUB_REFORMAT_OK"`.
- `EngineContext::with_backend` statt `EngineContext::new`.
- Assertions:
  - aufgezeichneter Call == Tool `ctx_refactor` mit `action == "reformat"`;
  - `args["path"]` == erwarteter absoluter Pfad (kein `MissingArg`);
  - Bridge-Output == `"STUB_REFORMAT_OK"` **verbatim** (Pass-Through-Transparenz).
- Die env-abhängige `BACKEND_REQUIRED || ERROR`-Assertion entfällt.

### 2. `src/engine.rs` — `reformat_renders_backend_required_e2e`

- **Umbenennen** → `reformat_dispatches_through_render_pipeline`.
- `EngineContext::with_backend` + denselben Stub; `render_body(ctx, "@reformat path=…")`.
- Assertions (erhaltener e2e-Wert über die volle Render-Pipeline):
  - **nicht** `unknown directive` (beweist: `@reformat` dispatcht durch die Pipeline,
    kein Unknown-Directive-Fallback);
  - Output **enthält** `"STUB_REFORMAT_OK"` (Pass-Through durch die volle Pipeline);
  - Output **nicht leer**.
- Die `BACKEND_REQUIRED`-spezifische Assertion entfällt.

### Mock-Platzierung

Inline-Stub je Test-Modul (folgt dem bestehenden `phases.rs`-Idiom). Kein neues
Shared-Test-Support-Modul — die Stubs sind je 2–3 Zeilen und die Duplizierung ist
geringer als die Kopplung, die ein geteiltes Modul einführen würde.

### Sentinel-Wahl

Neutrales `"STUB_REFORMAT_OK"` statt eines simulierten `BACKEND_REQUIRED`- oder
`via rustfmt`-Strings. Begründung: Der Test prüft **Transparenz des Bridges**
(reicht durch, was das Backend liefert) — nicht, *welchen* String lean-ctx emittiert.
Ein neutrales Sentinel macht diese Absicht explizit und vermeidet eine falsche
Kopplung an lean-ctx-Output-Formate.

## Bewusst NICHT enthalten (YAGNI)

- **Kein PATH-Mangeln** (`rustfmt` aus dem Test-PATH entfernen) — erzwingt zwar
  Headless-Degradierung, koppelt den Test aber weiter an lean-ctx-Verhalten und ist
  plattform-/toolchain-fragil.
- **Keine tolerante Multi-Pfad-Assertion** (`BACKEND_REQUIRED || via rustfmt || ERROR`)
  — hielte den Test e2e-gekoppelt an lean-ctx; widerspricht der Entscheidung.
- **Kein neuer Live-Degradierungstest** — echte Degradierung ist lean-ctx-Vertrag.

## Akzeptanzkriterien

- `cargo nextest run` ist grün **mit** `rustfmt` im PATH (Default-Dev-Maschine).
- `cargo nextest run` ist grün **ohne** `rustfmt` im PATH (beide Tests sind nun
  backend-agnostisch).
- Kein Test in `src/bridges/reformat.rs` / `src/engine.rs` ruft für diese zwei Fälle
  mehr das real installierte `lean-ctx`-Binary auf.
- Zero clippy warnings; `cargo fmt` vor Commit angewendet.
- Output-Determinismus (#498) unberührt — Änderungen sind test-only.
