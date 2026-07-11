# ctx_patch-Integration in Hard-Rules, Skills & Hooks — Design

**Datum:** 2026-07-11
**Branch:** feat-lmd-v2
**Status:** Design (approved) → bereit für Plan

## Problem

lean-ctx hat seit 3.9.0 `ctx_patch` (Anchored-Editing, „Edit Loop v1", #1008): lies
mit `ctx_read(mode=anchored)` die `LINE:HASH`-Anker, patche dann per Anker — der
Agent reproduziert nie den alten Text byte-für-byte (~41 % weniger Argument-Output-
Tokens, Reliability 10/10 vs. str_replace 5/10). Claude Code bekommt `ctx_patch`
advertised (Standard-Profil, 16 Tools).

Im lean-md-Repo kommt `ctx_patch` **0×** vor. Die gesamte Edit-Guidance zeigt noch
auf `ctx_edit` / native Edit / `ctx_refactor`. Folge: In den letzten Sessions wurde
`ctx_patch` nie genutzt. Der **aktive Fehl-Lenker** ist der Hook
`edit-tool-discipline.py` — er verweigert native Edit und lenkt in seinen
Deny-Reasons ausschließlich auf `ctx_edit`/`ctx_refactor`, nennt `ctx_patch` nie.

## Ziel

`ctx_patch` (Anchored-Loop) wird **erste Wahl für alle Non-Symbol-Edits** — in der
Guidance-Schicht (Content-Seeds), in den Repo-Instruktionen und in den Hooks.
`ctx_edit` bleibt dokumentierter Ausnahme-Fallback, `ctx_refactor` behält
Symbol-Edits.

## Nicht-Ziele / Entscheidungen

- **Kein `@patch`-Bridge, kein Directive-Token.** Anchored-Editing ist zweistufig
  und session-live (Anker = Hash der aktuellen Zeile); eine atomare Bridge wie
  `@edit`→`ctx_edit` kann nie gültige Anker vorab tragen. Der Wert liegt auf der
  Tool-Ebene (Agent ruft das `ctx_patch`-Tool interaktiv). ∴ **keine** Änderung an
  `bridges/*`, `render.rs` (`WORK_DIRECTIVES`), `availability.rs`,
  `gloss/directives.lmd.md`.
- **Plan-authorbar via Recipe statt Bridge.** Ein `@define patch(...)`-Recipe
  kapselt den Anchored-Loop rein auf Content-Ebene (Schritt 1 = bestehendes
  `@read mode=anchored`, Schritt 2 = `ctx_patch`-Tool-Call in Prosa). Verifiziert:
  `@read` reicht `mode` unverändert an `ctx_read` durch (`bridges/read.rs:31`).
  `@call patch(...)` ruft das Recipe über den **bestehenden** `@call`-Directive auf
  — `patch` ist ein Recipe-Name, **kein** neues `@patch`-Directive-Token.
- Keine unbezogenen Refactorings.

## Ausnahmeregel — wann `@edit`/`ctx_edit` (str_replace) korrekt bleibt

1. **Tiny-Span** (1–2 Token): wenn der `LINE:HASH`-Anker mehr Tokens kostet als der
   ersetzte `old_string` (der A/B-Bench nennt diese Fälle explizit als Ausnahme).
2. **Replace-all** eines identischen Non-Symbol-Strings über verstreute Zeilen
   (`ctx_edit all=true`): ein Call statt N Einzel-Anker.
3. (Fußnote) Kein frischer Anchored-Read praktikabel — seltener Wegwerf-Randfall.

Symbol-Renames/Moves/Extracts bleiben **immer** `@refactor` (ctx_refactor), nie
`ctx_patch`.

## Betroffene Flächen

### A. Content-Seeds (byte-stabil → Rebless nötig)

1. **`content/templates/plan-recipes.lmd.md`** — neues Recipe (Kernstück):

   ```
   @define patch(path, target)
   <!-- Anchored non-symbol edit: @read anchored for LINE:HASH, then ctx_patch by anchor (no old-text recall) -->
   1. Run: `@read {{ path }} mode=anchored` — LINE:HASH anchors for {{ target }}.
   2. Apply via `ctx_patch` (op by line+hash anchor) — never re-emit old text.
      Exception → `@edit`: tiny-span (1–2 tok, anchor ≥ old_string) or replace-all.
   @define-end
   ```

   Erste Body-Zeile ist HTML-Comment → Index-Completeness-Gate erfüllt.

2. **`content/templates/plan-template.lmd.md`** — neuer Edit-Slot in den
   Conditional slots: „Non-Symbol-Edit → `@call patch(path, target)`; Tiny-Span/
   replace-all → `@edit`; Symbol-Rename/Move/Extract → `@refactor`." Skills rendern
   Pläne aus diesem Template und erben `@call patch(...)`.

3. **`content/core/hard-rules.lmd.md`** — neue Always-on-Edit-Regel: Non-Symbol →
   `ctx_read(mode=anchored)` → `ctx_patch`; `ctx_edit` nur Tiny-Span/replace-all;
   `@refactor` für Symbole.

4. **`content/tooling/mcp-tools.lmd.md`** — Anchored-Edit-Loop + Ausnahmeregel
   dokumentieren.

5. **`content/lang/rust.lmd.md`** — Non-Symbol-Edits → anchored `ctx_patch`;
   `@edit` als Ausnahme; `@refactor` für Symbole (bestehende Regel bleibt).

6. **`content/core/dispatch-contract.lmd.md`** (Z.14) — Subagent-Regel umschreiben:
   „Rust non-symbol edits → `@read mode=anchored` → `ctx_patch`; `ctx_edit` nur
   Ausnahme; symbol nav/refactor → `ctx_refactor`/`@symbol`."

7. **`content/core/_fragments/tool-quick-ref.lmd.md`** — optionale Ein-Zeilen-Notiz
   zum Edit-Pfad; `@edit=ctx_edit` bleibt (kein neuer Directive).

### B. Repo-Doku (nicht gerendert, aber vom Agenten gelesen)

8. **`AGENTS.md`** — Z.16 („File editing"), Z.25/26 (Tool-Tabelle), Z.95: `ctx_patch`
   als primären Edit-Pfad (anchored), `ctx_edit` als Fallback ergänzen.

9. **`CLAUDE.md`** (Projekt) — Delta-Note: anchored-`ctx_patch`-first als
   Projekt-Regel echoen (die native-Edit-Sprache entsprechend relativieren).

### C. Hooks (`~/.claude/hooks/` + `~/.claude/settings.json`, außerhalb Repo)

10. **`edit-tool-discipline.py`** — `REASON_RS_EDIT` und `REASON_NONRUST_EDIT`
    umschreiben: primär `ctx_read(mode="anchored")` → `ctx_patch`; `ctx_edit`
    (str_replace) als Tiny-Span/replace-all-Fallback; `ctx_refactor` für
    Symbol-Edits. `REASON_WRITE_EXISTS` analog anpassen.

11. **`lean-ctx-policy-guard.py`** — `mcp__lean-ctx__ctx_patch` / `ctx_patch` in
    `EDIT_TOOLS` aufnehmen (schließt die config.toml-Schutzlücke: ctx_patch könnte
    sonst `shell_allowlist` ungehindert editieren).

12. **`~/.claude/settings.json`** — PreToolUse-Matcher des `lean-ctx-policy-guard.py`
    um `mcp__lean-ctx__ctx_patch` erweitern (sonst greift #11 nicht).

## Verifikation

- **Rebless-Ritual (#498):** Seed-Edits ändern die `include_str!`-Fragmente →
  Fragment-Consistency-Gate (built-in == on-disk) grün halten; `skills.sha256` +
  `skills.ctxpkg-hash` neu blessen (vgl. Commit `0d0fd72`).
- **Index-Completeness-Gate:** `lean-md render plan-recipes.lmd.md --signatures`
  muss das neue `patch`-Recipe mit Beschreibung listen.
- **Render-Smoke:** jede berührte Phase/Skill via
  `cargo run -q --bin lean-md -- render --skill <s> --phase <p> --consumer=ai` —
  non-empty, kein Eval-Error, byte-stabil über zwei Läufe.
- `cargo nextest run` (Fragment- + Bridge-Tests grün), `cargo clippy` zero-warnings,
  `cargo fmt` vor jedem `git add`.
- **Hook-Test manuell:** native Edit auf `.rs` und Non-Rust triggern → Deny-Reason
  nennt jetzt anchored `ctx_patch` zuerst; ctx_patch-Edit an config.toml mit
  `shell_allowlist` → Policy-Guard verweigert.

## Risiken

- **Byte-Stabilität:** vergessenes Rebless bricht das Fragment-Gate — im Plan als
  fester Verify-Slot verankert.
- **`@read mode=anchored`-Passthrough:** verifiziert (bridges/read.rs), kein Risiko.
- **Hooks außerhalb Repo:** nicht git-versioniert im lean-md-Repo; Änderung an
  `~/.claude/` ist Umgebungs-lokal — im Plan als eigener Task mit manuellem
  Smoke-Test, nicht durch die Repo-Test-Suite abgedeckt.
