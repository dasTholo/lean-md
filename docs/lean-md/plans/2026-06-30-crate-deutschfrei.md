# Crate komplett deutschfrei — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sämtliche deutschsprachigen Inhalte in `content/` (ohne `content/skills`), `src/` und `tests/` auf Englisch umstellen, ohne Funktions- oder Test-Verlust.

**Architecture:** Output-rendernde Funktionen (Gloss-Templates, `human_legend`, CLI-Strings) sind test-gekoppelt → Code/Template + Assertion atomar pro Task. Reine Doc-/Code-Kommentare werden nach Datei-Gruppen übersetzt. Ein finaler crate-weiter Scan (Umlaute **und** deutsche Funktionswörter = 0) ist das Vollständigkeits-Gate.

**Tech Stack:** Rust (lib `lean_md` + bin `lean-md`), `cargo nextest`, lean-ctx MCP tooling (`ctx_read`/`ctx_edit`/`ctx_search`/`ctx_shell`).

## Global Constraints

- Tests immer `cargo nextest run`, nie `cargo test`.
- Shell: kein `&&`/`||`/`;`-Chaining — jeder Befehl eine eigene Invocation.
- Vor jedem `git add` (pro geänderter Rust-Datei): `cargo fmt`.
- Determinismus (#498): keine Timestamps/Counter/Random in Output-Bodies; `include_str!`-Seeds bleiben byte-konsistent.
- `directives.lmd.md` ist `include_str!`-eingebettet (`gloss.rs:11`); Gate `embedded_table_matches_on_disk_file` bleibt nach Recompile selbst-konsistent.
- Bewusst ausgenommen: `content/skills/**`, `docs/**` (Spec-Quelle). `§`-Referenzen in Kommentaren bleiben; nur deutsche Zitat-Titel werden übersetzt.
- Code/Kommentare: Englisch. Diese Plan-Datei selbst bleibt Deutsch (Interaktions-Sprache).

---

## File Structure

| Datei | Verantwortung | Klasse |
|---|---|---|
| `content/gloss/directives.lmd.md` | Gloss-Template-Tabelle (eingebettet) | A/B |
| `src/gloss.rs` | Gloss-Render + Tests (header-name- & template-gekoppelt) | B |
| `src/crp.rs` | `human_legend` (`consumer=human` Output) + Tests | B |
| `src/bin/lean_md.rs` | CLI user-facing Strings + Doc-Kommentare | B/C |
| `content/tooling/availability-audit.md` | Audit-Doku (Prosa) | A |
| `src/availability.rs` | Doc-Header-Kommentar | C |
| `src/render.rs` | Multibyte-Test-Fixture + Doc-Kommentare | D/C |
| `src/bridges/{dispatch,handoff,smells}.rs` | Doc-/Code-Kommentare | C |
| `src/{engine,fragments,phases,macros,audit}.rs` + restliche `src/**` | Doc-/Code-Kommentare | C |

---

### Task 1: Gloss-Templates + gekoppelte gloss.rs-Strings/Tests (Klasse A1 + B1)

**Files:**
- Modify: `content/gloss/directives.lmd.md` (komplett)
- Modify: `src/gloss.rs:1` (Header-Doc), `:33` (Header-Name-Match), `:61` (Fallback), `:108-123` (4 Assertions), `:128` (Fallback-Assertion), `:135` (Header-Name-Assertion)

**Interfaces:**
- Produces: Gloss-Output-Strings, auf die `render::dispatch` (`consumer=human`) baut. Nach diesem Task lauten sie Englisch gemäß Mapping unten.

**Verbindliches Template-Mapping** (Template ↔ Assertion müssen wortgleich sein):

- [ ] **Step 1: `content/gloss/directives.lmd.md` vollständig ersetzen**

```markdown
<!-- lmd Phase 9 gloss table (D-5). Format: 2-column markdown table.
     Key = directive name or `name:op`. Slots: {N}=positional N, {raw}=all args,
     {key}=named arg. Lookup order: name:op → name → generic fallback. -->

| Directive        | Gloss template                         |
| read             | Read file `{0}`                        |
| search           | Search for `{0}`                       |
| list             | List directory `{0}`                   |
| query            | Run: `{raw}`                           |
| find             | Semantic search: `{raw}`               |
| symbol:refs      | Resolve references of `{1}`            |
| symbol:def       | Find definition of `{1}`               |
| symbol:impl      | Find implementations of `{1}`          |
| symbol:overview  | Symbol overview of `{1}`               |
| symbol           | Symbol analysis: `{raw}`               |
| graph:dependents | Resolve dependents of `{dependents}`   |
| graph:callers    | Resolve callers of `{callers}`         |
| graph:callees    | Resolve callees of `{callees}`         |
| graph            | Graph analysis: `{raw}`                |
| edit             | Apply code change                      |
| repomap          | Build repo map                         |
| impact           | Impact analysis for `{0}`              |
| architecture     | Architecture overview                  |
| outline          | Outline of `{0}`                       |
| routes           | List routes                            |
| smells           | Check code smells                      |
| review           | Code review                            |
| inspect          | Run inspections                        |
| count            | Count: `{raw}`                         |
| refactor         | Refactor: `{raw}`                      |
| reformat         | Format code: `{0}`                     |
```

- [ ] **Step 2: `src/gloss.rs` Header-Doc (Zeile 1) übersetzen**

`ctx_edit`: `//! Phase-9 human-readable gloss: directive name+args → German prose.`
→ `//! Phase-9 human-readable gloss: directive name+args → English prose.`

- [ ] **Step 3: Header-Name-Match (Zeile 33) anpassen**

Die Tabelle hat jetzt Header `Directive` statt `Direktive` — der Skip-Check muss mit:
`ctx_edit`: `|| key.eq_ignore_ascii_case("direktive")`
→ `|| key.eq_ignore_ascii_case("directive")`

- [ ] **Step 4: Generischen Fallback (Zeile 61) übersetzen**

`ctx_edit`: `None => format!("Direktive `@{name}`: `{}`", args.raw().trim()),`
→ `None => format!("Directive `@{name}`: `{}`", args.raw().trim()),`

- [ ] **Step 5: Test-Assertions (Zeilen 108–123) ersetzen**

```rust
    #[test]
    fn glosses_common_work_directives() {
        assert_eq!(
            gloss("read", "src/parser/block.rs"),
            "Read file `src/parser/block.rs`"
        );
        assert_eq!(
            gloss("query", "\"cargo nextest run\""),
            "Run: `cargo nextest run`"
        );
        assert_eq!(
            gloss("graph", "dependents=parse_block"),
            "Resolve dependents of `parse_block`"
        );
        assert_eq!(
            gloss("symbol", "refs parse_block"),
            "Resolve references of `parse_block`"
        );
    }
```

- [ ] **Step 6: Fallback-Assertion (Zeile 128) übersetzen**

`ctx_edit`: `assert_eq!(gloss("frobnicate", "x y"), "Direktive `@frobnicate`: `x y`");`
→ `assert_eq!(gloss("frobnicate", "x y"), "Directive `@frobnicate`: `x y`");`

- [ ] **Step 7: Header-Name-Assertion (Zeile 135) anpassen**

`ctx_edit`: `assert!(!t.contains_key("Direktive"), "header row skipped");`
→ `assert!(!t.contains_key("Directive"), "header row skipped");`

- [ ] **Step 8: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 9: Tests ausführen (gloss-Modul + Embed-Gate)**

Run: `cargo nextest run gloss`
Expected: PASS — insbesondere `glosses_common_work_directives`, `unknown_directive_uses_generic_fallback`, `table_parses_nonempty_and_skips_header`, `embedded_table_matches_on_disk_file`.

- [ ] **Step 10: Commit**

```bash
git add content/gloss/directives.lmd.md src/gloss.rs
git commit -m "i18n(gloss): English gloss templates + coupled gloss.rs strings/tests"
```

---

### Task 2: human-legend Output + Tests (Klasse B2)

**Files:**
- Modify: `src/crp.rs:45-47` (Doc), `:54-81` (Wörter + Notation-Label), `:148/:161/:176` (Assertions)

**Interfaces:**
- Produces: `human_legend()` liefert englische Wörter; `consumer=human`-Legende lautet `**Notation used:** …`.

- [ ] **Step 1: Doc-Kommentar (Zeilen 45–47) übersetzen**

`ctx_edit`: `/// the SAME kind buckets, expanded to German words (no dense glyphs). Used by`
→ `/// the SAME kind buckets, expanded to English words (no dense glyphs). Used by`

- [ ] **Step 2: Wort-Buckets (Zeilen 54–81) übersetzen**

```rust
    if has(&|s| matches!(s.kind, "fn" | "method")) {
        parts.push("Function");
    }
    if has(&|s| matches!(s.kind, "class" | "struct")) {
        parts.push("Class/Struct");
    }
    if has(&|s| matches!(s.kind, "interface" | "trait")) {
        parts.push("Trait/Interface");
    }
    if has(&|s| s.kind == "type") {
        parts.push("Type");
    }
    if has(&|s| s.kind == "enum") {
        parts.push("Enum");
    }
    if has(&|s| matches!(s.kind, "const" | "let" | "var")) {
        parts.push("Value/Constant");
    }
    if has(&|s| s.is_exported) {
        parts.push("public");
    }
    if has(&|s| s.is_async) {
        parts.push("async");
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("**Notation used:** {}", parts.join(", "))
    }
```

- [ ] **Step 3: Test-Assertions übersetzen**

- Zeile 148: `assert!(legend.contains("Funktion"), "fn → Funktion: {legend}");`
  → `assert!(legend.contains("Function"), "fn → Function: {legend}");`
- Zeile 161–163: `legend_e.contains("öffentlich"),` / `"is_exported → öffentlich: {legend_e}"`
  → `legend_e.contains("public"),` / `"is_exported → public: {legend_e}"`
- Zeile 176–177: `legend_a.contains("asynchron"),` / `"is_async → asynchron: {legend_a}"`
  → `legend_a.contains("async"),` / `"is_async → async: {legend_a}"`

- [ ] **Step 4: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 5: Tests ausführen**

Run: `cargo nextest run crp`
Expected: PASS — `human_legend_expands_glyphs_to_words`, `human_legend_empty_for_no_sigs`.

- [ ] **Step 6: Commit**

```bash
git add src/crp.rs
git commit -m "i18n(crp): English human_legend output + coupled tests"
```

---

### Task 3: CLI-Strings + Doc-Kommentare in lean_md.rs (Klasse B3/C)

**Files:**
- Modify: `src/bin/lean_md.rs` (alle deutschen `eprintln!`/`println!`-Strings + Doc-Kommentare)

**Interfaces:**
- Produces: keine test-gekoppelten Strings bekannt; rein user-facing/Kommentar.

- [ ] **Step 1: Bekannten CLI-String (Zeile 343) übersetzen**

`ctx_edit`: `eprintln!("{} existiert bereits — nicht überschrieben", p.display());`
→ `eprintln!("{} already exists — not overwritten", p.display());`

- [ ] **Step 2: Datei auf weitere deutsche Strings/Kommentare scannen**

Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src/bin/lean_md.rs`
Dann Pattern `\b(der|die|das|und|nicht|wird|für|über|muss|bereits|nicht)\b` path `src/bin/lean_md.rs`.
Jeden Treffer (Kommentar oder String) ins Englische übersetzen via `ctx_edit`. Erwartet: nach Step 1 ggf. 0 weitere; falls Treffer, einzeln übersetzen.

- [ ] **Step 3: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 4: Build + Tests**

Run: `cargo nextest run`
Expected: PASS (keine Regression; bin baut).

- [ ] **Step 5: Verifikations-Scan der Datei = 0**

Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src/bin/lean_md.rs`
Expected: 0 Treffer.

- [ ] **Step 6: Commit**

```bash
git add src/bin/lean_md.rs
git commit -m "i18n(cli): English user-facing strings + comments in lean_md.rs"
```

---

### Task 4: availability-audit.md + availability.rs-Header (Klasse A2 + C)

**Files:**
- Modify: `content/tooling/availability-audit.md` (komplett, Prosa)
- Modify: `src/availability.rs:1` (Doc-Header mit Spec-Zitat)

**Interfaces:**
- Consumes: COVERAGE-Tabelle bleibt strukturell identisch (nur Prosa übersetzt).

- [ ] **Step 1: `content/tooling/availability-audit.md` ins Englische übersetzen**

Übersetze sämtliche Prosa und Überschriften (Tabellen-Strukturen/Direktiv-Namen/Tool-Namen bleiben). Beispiele:
- `# Tool-Verfügbarkeits-Audit — Brainstorming-Pfad (Phase 10)` → `# Tool availability audit — brainstorming path (phase 10)`
- `Coverage-Matrix: jeder Brainstorming-Workflow-Schritt → lmd-Direktive → lean-ctx-Backing.` → `Coverage matrix: each brainstorming workflow step → lmd directive → lean-ctx backing.`
- `Quelle der Wahrheit ist …` → `The source of truth is …`
- `## Bewusst NICHT im Brainstorming-Pfad (Gap-Liste, transparent)` → `## Deliberately NOT in the brainstorming path (gap list, transparent)`
- `**Bewusster Gap:**` → `**Deliberate gap:**`, `Die Test-Ausführung …` → `Test execution …`
- `Die green-Phase dispatcht …` → `The green phase dispatches …`
Tabellen-Spaltenköpfe `| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing |` → `| Workflow step | lmd directive | lean-ctx backing |`. Alle deutschen Sätze vollständig übersetzen.

- [ ] **Step 2: `src/availability.rs:1` Spec-Zitat-Titel übersetzen, §-Referenz behalten**

`ctx_edit`: `//! Tool-availability audit (Spec §5.4 "Tool-Verfügbarkeits-Audit" + §8 #12).`
→ `//! Tool-availability audit (Spec §5.4 "Tool availability audit" + §8 #12).`
(Falls weitere deutsche Kommentare in `availability.rs`: per Scan in Step 4 erfassen.)

- [ ] **Step 3: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 4: Tests + Scan**

Run: `cargo nextest run availability`
Expected: PASS (`every_covered_directive_is_registered`, `coverage_carries_*`).
Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `content/tooling/availability-audit.md`
Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src/availability.rs`
Expected: jeweils 0 Treffer.

- [ ] **Step 5: Commit**

```bash
git add content/tooling/availability-audit.md src/availability.rs
git commit -m "i18n(audit): English availability-audit doc + availability.rs header"
```

---

### Task 5: render.rs Multibyte-Fixture + Doc-Kommentare (Klasse D + C)

**Files:**
- Modify: `src/render.rs:394` (Fixture), `:400` (starts_with), `:404` (contains); `:189/:191/:217` + weitere Doc-Kommentare

**Interfaces:**
- Produces: Test prüft weiterhin Multibyte-Splice — mit neutralen Zeichen `☃`, Em-Dash `—`, `∆` statt deutscher Umlaute.

- [ ] **Step 1: Fixture-String (Zeile 394) ersetzen**

`ctx_edit`: `let source = "Grüße {{ date }} — Größe äöü\n";`
→ `let source = "Greetings ☃ {{ date }} — size ∆\n";`

- [ ] **Step 2: Assertion `starts_with` (Zeile 400) anpassen**

`ctx_edit`: `out.starts_with("Grüße "),`
→ `out.starts_with("Greetings "),`

- [ ] **Step 3: Assertion `contains` (Zeile 404) anpassen**

`ctx_edit`: `out.contains("Größe äöü"),`
→ `out.contains("size ∆"),`

- [ ] **Step 4: Deutsche Doc-Kommentare in render.rs übersetzen**

Run: `ctx_search` Pattern `\b(der|die|das|und|nicht|wird|werden|für|über|muss|Direktiven|führt|aufgelöst)\b` path `src/render.rs`
Übersetze jeden Kommentar-Treffer (u. a. ~Z.189, 191, 217: „Work-Klasse … diese Direktiven führt der SUBAGENT …", „… `{{ }}`-Inline … ist Template-Klasse → eager aufgelöst", „… werden aufgelöst") ins Englische via `ctx_edit`.

- [ ] **Step 5: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 6: Tests + Scan**

Run: `cargo nextest run render`
Expected: PASS — `splice_preserves_multibyte_prose_around_directive`.
Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src/render.rs`
Expected: 0 Treffer (☃/—/∆ sind keine Umlaute).

- [ ] **Step 7: Commit**

```bash
git add src/render.rs
git commit -m "i18n(render): neutral multibyte test fixture + English comments"
```

---

### Task 6: bridges-Kommentare (Klasse C)

**Files:**
- Modify: `src/bridges/dispatch.rs`, `src/bridges/handoff.rs`, `src/bridges/smells.rs` (+ Scan über `src/bridges/**`)

**Interfaces:**
- Keine Funktions-/Test-Kopplung — reine Kommentar-Übersetzung.

- [ ] **Step 1: Deutsche Kommentare in den drei Dateien übersetzen**

Bekannte Stellen:
- `dispatch.rs:3-4` (`//! … Kein Spawn, kein ctx_agent/ctx_handoff-Aufruf — der Baton ist Instruktions-Text im Contract.`), `:12-13` (`/// … lädt die deferred lazy-core- /// Tools vor dem ersten Read im Subagenten. Byte-stabil (#498).`), `:48`, `:224-225`.
- `handoff.rs:48-49`, `:65`, `:82`.
- `smells.rs:7` (`//! `text` — "erben, nicht neu erfinden", §5).`).
Jeden ins Englische übersetzen via `ctx_edit` (Direktiv-/Tool-/Spec-Referenzen behalten).

- [ ] **Step 2: Gesamtes `src/bridges/` auf Restbestände scannen**

Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src/bridges` (max_results 200)
Run: `ctx_search` Pattern `\b(der|die|das|und|nicht|wird|werden|für|über|muss|bleibt|gegen|erbt|zwingt|nachschlagbar)\b` path `src/bridges` (max_results 200)
Jeden weiteren Treffer übersetzen. Expected nach Übersetzung: 0.

- [ ] **Step 3: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 4: Tests**

Run: `cargo nextest run`
Expected: PASS (keine Regression).

- [ ] **Step 5: Commit**

```bash
git add src/bridges
git commit -m "i18n(bridges): English doc/code comments"
```

---

### Task 7: Kern-Module + restliche src-Kommentare (Klasse C)

**Files:**
- Modify: `src/engine.rs`, `src/fragments.rs`, `src/phases.rs`, `src/macros.rs`, `src/audit.rs` + alle übrigen `src/*.rs` mit deutschen Kommentaren (außer in Task 1–6 erledigte)

**Interfaces:**
- `fragments.rs` enthält test-nahe Kommentare (z. B. `:112`, `:154`, `:165`) — nur Kommentar-Text, keine Assertion-Strings; Tests bleiben unberührt.

- [ ] **Step 1: Bekannte Kern-Stellen übersetzen**

- `engine.rs:41-43` (`/// Name nachschlagbar für `@dispatch` … Render-/lifecycle-frei — die /// Work-Bridges bleiben verbatim … Getrennt von `phase_scope` /// (das den Inline-Render-Lifecycle trägt).`)
- `fragments.rs:13-15` (`/// `@dispatch` render … Portiert aus … bleiben verbatim — die `DispatchBridge``), `:112` (`// Parametrisierung bleibt verbatim — Substitution ist Sache der DispatchBridge.`), `:154` (`// D-8: serena/jetbrains wurden entfernt; der Kanon darf sie nicht mehr nennen.`), `:165` (`// Die heutigen Backings müssen genannt sein.`)
- `phases.rs:439-457` (mehrere `///`/`//`: „Render-FREI und lifecycle-FREI …", „Phasen: nicht verschachtelt; …", „… hier nur echte Öffner.")
- `macros.rs:83` (`/// (spec §4 "textuelle {{ p }}-Interpolation im Body-Content"). Missing args`)
- `audit.rs:30` (`/// Rough bridge-size estimate in lines (spec §3.1 "Bridge-Zeilenschätzung").`)
Jeden via `ctx_edit` übersetzen (Spec-§/Direktiv-Namen behalten; deutsche Zitat-Titel wie „Bridge-Zeilenschätzung" übersetzen).

- [ ] **Step 2: Restliches `src/` (ohne bridges, ohne Task-1–5-Dateien) scannen**

Pro Datei-Gruppe (Top-Level `src/*.rs`, `src/parser/`) scannen und übersetzen:
Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src` (max_results 200) — Treffer aus bereits committeten Dateien ignorieren (Klasse-D-Zeichen ☃/∆ tauchen nicht als Umlaut auf).
Run: `ctx_search` Pattern `\b(der|die|das|und|nicht|wird|werden|für|über|muss|müssen|bleibt|bleiben|wurde|wurden|gegen|trägt|verbietet)\b` path `src` (max_results 200).
Jeden Kommentar-/String-Treffer übersetzen via `ctx_edit`.
**Ausnahme bestätigen:** `crp.rs` „öffentlich"/„Funktion" etc. sind in Task 2 bereits englisch; falls hier noch Treffer in `crp.rs` → Task 2 nacharbeiten.

- [ ] **Step 3: `cargo fmt`**

Run: `cargo fmt`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 4: Tests**

Run: `cargo nextest run`
Expected: PASS (keine Regression).

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "i18n(core): English doc/code comments across remaining src modules"
```

---

### Task 8: Finaler crate-weiter Verifikations-Scan (Definition of Done)

**Files:**
- Read-only Verifikation über `src/**`, `content/**` (ohne `content/skills`)

- [ ] **Step 1: Umlaut-Scan = 0**

Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `src` (max_results 300)
Run: `ctx_search` Pattern `[äöüßÄÖÜ]` path `content/gloss` ; dann `content/tooling` ; `content/core` ; `content/lang` ; `content/templates`
Expected: 0 Treffer überall. (Klasse-D-Zeichen ☃/—/∆ sind keine Umlaute → erscheinen nicht.)

- [ ] **Step 2: Deutsche-Funktionswort-Scan = 0**

Run: `ctx_search` Pattern `\b(der|die|das|den|dem|und|nicht|wird|werden|muss|müssen|sind|eine|einen|kein|keine|wenn|dann|hier|nur|für|über|von|mit|auf|zum|zur|bleibt|wurde|gegen)\b` path `src` (max_results 300)
Run: dito path `content/gloss`, `content/tooling`, `content/core`, `content/lang`, `content/templates`.
Expected: 0 Treffer in Kommentaren/Strings. Jeder verbleibende Treffer wird übersetzt; danach Scan wiederholen, bis 0.

- [ ] **Step 3: Clippy = 0 Warnungen**

Run: `cargo clippy --all-targets`
Expected: Exit 0, keine Warnungen.

- [ ] **Step 4: Format-Check**

Run: `cargo fmt --check`
Expected: Exit 0, keine Diffs.

- [ ] **Step 5: Voller Testlauf**

Run: `cargo nextest run`
Expected: alle Tests PASS.

- [ ] **Step 6: Commit (falls Step 2 Nacharbeiten erforderte; sonst entfällt)**

```bash
git add -A
git commit -m "i18n: final German-free sweep (src + content)"
```

---

## Self-Review (vom Plan-Autor durchgeführt)

- **Spec-Coverage:** Klasse A → Task 1 (gloss) + Task 4 (audit). Klasse B → Task 1/2/3. Klasse C → Task 3/4/5/6/7. Klasse D → Task 5. DoD §7 → Task 8. Terminologie §4 → in Task 1/2 verbindlich eingebettet. Determinismus §5 → Global Constraints + Task-1-Gate. Keine Lücke.
- **Neu gegenüber Spec:** Tabellen-Header-Kopplung `gloss.rs:33` + `:135` (Header `Direktive`→`Directive`) — in Task 1 ergänzt.
- **Type-/String-Konsistenz:** Template-Werte in `directives.lmd.md` (Task 1 Step 1) sind wortgleich zu den Assertions (Task 1 Step 5/6); `human_legend`-Wörter (Task 2 Step 2) wortgleich zu Assertions (Task 2 Step 3); render-Fixture (Task 5 Step 1) wortgleich zu Assertions (Task 5 Step 2/3).
