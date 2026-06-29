# Skill-Body-Variablen (`@var` + `.lean-ctx/lean-md/vars.toml`) — Design

> **Status:** Spec (brainstormed, freigegeben). Nächster Schritt: `writing-plans`.
> **Branch:** `feat-lmd-v2`. **Baut auf:** Spec #1 (lmd-test-driven-development + Skill-Platform-Fundament, Commits
`72e1e4d..f6b8830`).

## Goal

Skill-Bodies sollen **konfigurierbare Variablen** tragen, statt projektspezifische Werte
(z.B. den Test-Befehl) hartzukodieren. Der Skill-Autor deklariert eine Variable mit Default
im Body; ein projekt-lokales, **gitignored** `.lean-ctx/lean-md/vars.toml` überschreibt den
Default zur Render-Zeit. Erstnutzer: der Test-Befehl in `lmd-test-driven-development`
(Default `cargo test`, dieses Projekt überschreibt zu `cargo nextest run`).

Dies nutzt **bewusst** den `.lean-ctx/lean-md/`-Overlay-Pfad als realen End-to-End-Test
(Schwester zum D7 Body-Override aus Spec #1).

## Architecture

Ein neuer **`@var`-Bridge** spielt — exakt wie `@include`/`{{ include }}` — zwei Rollen,
abhängig vom Aufruf-Kontext:

| Rolle           | Syntax                           | Kontext                         | Rendert zu                                                                       |
|-----------------|----------------------------------|---------------------------------|----------------------------------------------------------------------------------|
| **Deklaration** | `@var NAME default="…" desc="…"` | Block (zeilenführend), Body-Top | *(leer)* — registriert Default in `ctx.vars`, falls Config den Key nicht liefert |
| **Verwendung**  | `{{ var NAME }}`                 | Inline (in den Phasen)          | der aufgelöste Wert aus `ctx.vars[NAME]`                                         |

Werte landen in einer neuen **`EngineContext.vars`**-Map mit Präzedenz (niedrig→hoch):

1. **`@var …default=`** im Seed-Body → eingebauter Default.
2. **`.lean-ctx/lean-md/vars.toml`** (jailed gelesen) → Projekt-Override.

> **Render-Arg-Ebene (`--var key=val` / `ctx_md_render` `vars`-Objekt) ist bewusst DEFERRED.**
> Sie würde die `render_skill`-Signatur brechen (~10 Test-Call-Sites) und die gerade
> stabilisierte `src/bin/lean_md.rs` erneut anfassen. Der konkrete Bedarf ist mit
> `config > default` voll gedeckt. Additive Erweiterung später (siehe §Folge-Punkte).

### Warum `{{ var NAME }}` (Inline), nicht bare `@var NAME`

Der Inline-Parser (`src/parser/inline.rs`) triggert ausschließlich auf `{{ ` und claimt
`{{ name args }}`. `@` ist der **Block**-Trigger (Zeilenanfang). Registrierte Direktiven
sind dual und dispatchen inline über das `{{ }}`-Wrapper (`inline_known_directive_still_dispatches`,
`engine.rs:802` — *"the value tier must NOT shadow a registered inline directive"*). Deshalb:

- `@var test_cmd default="cargo test"` (Block) deklariert,
- `{{ var test_cmd }}` (inline) dispatcht den `var`-Bridge → Lookup.

Kein separater `{{ var.X }}`-evalexpr-Namespace nötig (der Bridge-Dispatch erledigt die
Auflösung; spart `macros.rs`-Namespace-Arbeit).

## Phasen-Isolation (der Knackpunkt)

Die `@var …default=`-Deklaration steht **einmal ganz oben** im Body, **außerhalb** jeder
Phase. `capture_phase_bodies` schließt sie damit natürlich aus den isolierten Phasen-Bodies
aus. Beim isolierten Phasen-Render (`render_skill(name, Some(phase), …)`) enthält die Phase
die Deklaration also nicht — wohl aber die `{{ var test_cmd }}`-Verwendung.

**Lösung — Pre-Pass auf dem vollen Body** (vor dem isolierten Phasen-Render):

1. Config `.lean-ctx/lean-md/vars.toml` laden (jailed) → `ctx.vars` (Override-Schicht).
2. **Vollen Body** nach `@var NAME default="…"` scannen; Default in `ctx.vars[NAME]` setzen,
   **nur** wenn die Config den Key nicht bereits geliefert hat.
3. Isolierte Phase rendern; `{{ var NAME }}` liest aus `ctx.vars`.

→ DRY (Default an genau einer Stelle), kein Cross-Phase-Leak, funktioniert für `phase=None`
(voller Body) wie für isolierte Phasen identisch.

## Components (kleine, isolierte Einheiten)

### Neu: `src/bridges/var.rs` — `@var`-Bridge

- `name() -> "var"`, registriert in `default_registry()` (`src/bridges/mod.rs`).
- `execute(ctx, args)`:
    - Hat `args` ein `default=`-Token → **Deklarations-Modus**: positional(0) = `NAME`;
      `default="…"` = Default; optionales `desc="…"` = Metadaten (render-irrelevant).
      Setzt `ctx.vars[NAME] = default`, **nur falls** `NAME` noch nicht in `ctx.vars`
      (Config-Präzedenz). Gibt **leeren String** zurück.
    - Sonst (nur `NAME`) → **Lookup-Modus**: gibt `ctx.vars[NAME]` zurück, oder `""` falls
      unbekannt (defensiv; ein unbekanntes `NAME` ist ein Autor-Fehler, kein Panic).
- Fängt versprengte Inline-`@var`/`{{ var … }}`-Deklarationen robust ab (Block-Deklaration
  ist der Normalfall; der Pre-Pass ist die autoritative Quelle für isolierte Phasen).

### Neu: `src/skill_vars.rs` — Config-Loader + Scanner + Init-Generator

- **`load_vars(jail_root) -> HashMap<String,String>`**: löst
  `<jail_root>/.lean-ctx/lean-md/vars.toml` via `crate::pathx::jail_path` (PathJail),
  liest + parst ein **flaches `key = "value"`-TOML-Subset von Hand** (kein `toml`-Dep):
    - eine Zuweisung pro Zeile, `key = "value"` (Wert in Double-Quotes), `#`-Kommentare,
      Leerzeilen ignoriert; nur String-Werte, flache Keys.
    - Datei absent / nicht jailed → leere Map (kein Fehler).
- **`scan_var_decls(body) -> Vec<VarDecl{ name, default, desc }>`**: extrahiert alle
  `@var NAME default="…" [desc="…"]`-Deklarationen aus dem **vollen** Body. Eine Quelle,
  zwei Nutzer: Render-Pre-Pass **und** Init-Generator.
- **`render_vars_template(decls) -> String`**: erzeugt den kommentierten `vars.toml`-Inhalt
  (pro Decl: `# NAME: <desc>` + `NAME = "<default>"`), plus Header-Kommentar.

### Geändert: `src/engine.rs`

- `EngineContext` bekommt `vars: RefCell<HashMap<String,String>>` (Interior Mutability,
  konsistent mit `param_scope`), plus Accessoren `var_get(name)` / `vars_seed(map)` /
  `var_set_default(name, val)` (Default-falls-absent).

### Geändert: `src/skills.rs` — `render_skill`-Pre-Pass

- Vor dem Render (beide Pfade): `ctx.vars_seed(load_vars(&jail_root))`, dann für jede
  `scan_var_decls(body)`-Deklaration `ctx.var_set_default(name, default)`.
- Reihenfolge: erst Config seeden (Override), dann `@var`-Defaults (nur falls absent).

### Geändert: `src/bin/lean_md.rs` — `skill vars --init [name]`

- Neuer Sub-Sub-Command unter dem bestehenden `skill`-Command. **Name ist optional**:
  - `lean-md skill vars --init <name>` → **ein** Skill (Spezialfall).
  - `lean-md skill vars --init` (ohne Name) → **alle** Skills aus der `SKILLS`-Registry,
    aggregiert in EINE projekt-globale `vars.toml`.
- Lädt die Skill-Body(s), scannt `@var`-Deklarationen (`scan_var_decls`), aggregiert sie
  (Dedup nach Name — siehe §Cross-Skill-Aggregation), rendert das Template, schreibt nach
  `.lean-ctx/lean-md/vars.toml` **nur falls absent** (kein Clobber von User-Edits);
  existiert die Datei → Template nach **stdout** + Hinweis „existiert bereits, nicht
  überschrieben" auf stderr (Exit 0).
- **Architektur-Constraint (vom Plan einzuhalten):** Scanner + Generator sind
  **skill-agnostisch** — `scan_var_decls(body)` per Body, `render_vars_template(decls)` über
  eine flache `Vec<VarDecl>`. Aggregation = „über `SKILLS` iterieren + Decls sammeln". Der
  Single-Skill-Pfad ist eine Iteration. Die All-Skills-Möglichkeit ist damit von Anfang an
  offen und **darf nicht zugemauert werden**.

### Geändert: `content/skills/lmd-test-driven-development/body.lmd.md`

- Direkt nach dem führenden HTML-Kommentar, **vor** `@phase "red"`:
  ```
  @var test_cmd default="cargo test" desc="Test runner command; this project uses 'cargo nextest run'"
  ```
- Jedes hartkodierte `cargo nextest run` in den Phasen RED/GREEN/REFACTOR →
  `{{ var test_cmd }}` (die `ctx_shell "…"`-Umrandung bleibt).
- `tdd_body_matches_seed_file_on_disk` (embedded == on-disk) bleibt grün (neuer, byte-
  identischer Inhalt).

## Lifecycle: wer erstellt `vars.toml`, wann

**Grundprinzip: die Datei ist ein OPTIONALER Override, nicht erforderlich.**

| Frage | Antwort |
|---|---|
| **Default-Fall** | **Niemand erstellt sie.** Absent → `@var`-Defaults greifen → Skill läuft zero-config (`cargo test`). |
| **Wer** | Entweder (a) der User editiert sie von Hand, oder (b) `lean-md skill vars --init [name]` generiert ein kommentiertes Template (opt-in, explizit). |
| **Wann** | Zur Projekt-Setup-Zeit, **nur** wenn man einen Default überschreiben will. |
| **NICHT bei `skill install`** | Install hat eine andere Verantwortung (materialisiert den `SKILL.md`-Discovery-Stub). Es erzeugt `vars.toml` **nicht** — optional nur ein Hinweis-Print „run `skill vars --init` to customize variables". |
| **NICHT beim Render** | Kein Auto-Create / kein Surprise-File. Render liest sie nur, falls vorhanden. |
| **Versionierung** | gitignored (`.lean-ctx/lean-md/`), projekt-lokal, nie committet. |
| **Clobber-Schutz** | `--init` schreibt nur falls absent; existiert sie → Template nach stdout (User merged selbst). |

**Wachstum (neue Skills, Spec #2/#3):** Da all-skills-aggregierbar, erzeugt `vars --init`
(ohne Name) eine frische Gesamtdatei über **alle** aktuellen Skills. Ein späteres
`--merge`/`--update` (fehlende Keys ergänzen ohne Clobber) ist ein Folge-Punkt — die
skill-agnostische Scanner/Generator-Struktur hält ihn offen.

## Cross-Skill-Aggregation

`vars.toml` ist **projekt-global + flach** — genau das ermöglicht, Vars aus **allen** Skills
in EINE Datei zu aggregieren (während `skill install` per-Skill bleibt). Dedup-Regel beim
Aggregieren mehrerer Skill-Bodies:

- **Gleicher Name, gleicher Default** (z.B. `test_cmd` in mehreren Skills) → EIN Eintrag
  (gewollt geteilt — ein projekt-globaler Wert wirkt auf alle Skills, die ihn nutzen).
- **Gleicher Name, divergierende Defaults** → EIN Eintrag (erstes Vorkommen gewinnt) **plus**
  ein Warn-Kommentar `# WARN: <name> hat divergierende Defaults über Skills (…)` in der
  generierten Datei — transparent, kein stilles Überschreiben. (Sauberere Auflösung =
  per-Skill-Vars, Folge-Punkt.)

## Data Flow

```
ctx_md_render(skill, phase)  /  lean-md render --skill --phase
        │
        ▼
render_skill(name, phase, …, jail_root)
        │  ① load_vars(jail_root)          → ctx.vars  (Override aus vars.toml, jailed)
        │  ② scan_var_decls(full body)     → ctx.var_set_default(…)  (Default falls absent)
        │  ③ capture_phase_bodies(body)    → isolierte Phase
        ▼
render_body(isolated phase)
        │   {{ var test_cmd }}  → @var-Bridge Lookup → ctx.vars["test_cmd"]
        ▼
   "… ctx_shell \"cargo nextest run\" …"   (lokal)  /  "… cargo test …"  (default)
```

## Error Handling

- `vars.toml` absent → leere Map, Defaults greifen. **Kein** Fehler (häufigster Fall).
- `vars.toml` außerhalb des Jails (Symlink/`..`) → `jail_path` lehnt ab → leere Map
  (Fallback auf Default), kein Panic. Server-seitiges Jailing bleibt autoritativ.
- Unparsbare Zeile in `vars.toml` → Zeile überspringen (best-effort), gültige Keys greifen.
- `{{ var UNKNOWN }}` (kein Decl, keine Config) → `""` (defensiv; Autor-Fehler, kein Abbruch).
- `skill vars --init <unknown>` → Fehler „unknown skill", Exit 1.

## Determinismus (#498)

Output bleibt eine **deterministische Funktion der Inputs** — jetzt inkl. des `vars.toml`-
Inhalts. Gleicher Body + gleiches `vars.toml` → byte-identisch; zwei Renders gleich
(`skill_render_is_byte_stable_and_isolated` bleibt gültig). Keine Timestamps/Counter/Random.
CliBackend == McpBackend (beide rufen dasselbe `render_skill`).

## Testing (Gates)

Alle hermetisch über temporäre `jail_root`-Verzeichnisse (wie die bestehenden Overlay-Tests).

1. **`@var`-Bridge registriert + dual:** Deklaration (`@var x default="d"`) rendert leer +
   setzt Default; `{{ var x }}` rendert den Wert. `default_registry().get("var").is_some()`.
2. **Config-Override:** RED-Phase rendert `cargo test` ohne `vars.toml`; `cargo nextest run`
   mit `<jail>/.lean-ctx/lean-md/vars.toml` (`test_cmd = "cargo nextest run"`) — **der reale
   `.lean-ctx/lean-md/`-Test**.
3. **Config-Präzedenz:** Config-Wert schlägt `@var`-Default (gleicher Key).
4. **Loader jailed:** `..`/absolute Escape → leere Map (kein Read außerhalb `jail_root`).
5. **TOML-Subset-Parse:** `key = "value"` + `#`-Kommentar + Leerzeile korrekt; Datei absent → leer.
6. **`scan_var_decls`:** extrahiert `(name, default, desc)` aus dem vollen Body.
7. **`vars --init <name>`:** erzeugt kommentierte Datei mit `desc` + Default; existiert sie →
   stdout, kein Clobber.
7b. **`vars --init` (all-skills):** aggregiert `@var`-Decls über **alle** `SKILLS`; geteilter
    Name (gleicher Default) → ein Eintrag; divergierende Defaults → ein Eintrag + Warn-Kommentar.
8. **Phasen-Isolation erhalten:** RED rendert `{{ var test_cmd }}` aufgelöst, **ohne**
   GREEN/REFACTOR-Marker-Leak.
9. **Determinismus #498:** zwei Renders mit gleichem `vars.toml` byte-identisch.
10. **Seed-Konsistenz:** `tdd_body_matches_seed_file_on_disk` grün (Body mit `@var`/`{{ var }}`).
11. **Quality-Bar:** `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` 0,
    `cargo nextest run` grün.

## Lokales Setup (dieses Repo)

`.lean-ctx/lean-md/vars.toml` (gitignored — `.lean-ctx/lean-md/` steht bereits in `.gitignore`):

```toml
# lmd skill vars — lmd-test-driven-development
# Werte editieren; Renders ziehen sie automatisch (Präzedenz: diese Datei > Seed-Default).

# test_cmd: Test runner command; this project uses 'cargo nextest run'
test_cmd = "cargo nextest run"
```

Erzeugbar via `lean-md skill vars --init lmd-test-driven-development` (dann Wert anpassen).

## Folge-Punkte (deferred, YAGNI)

- **Render-Arg-Ebene** `--var key=val` (CLI) + `vars`-Objekt (`ctx_md_render` MCP) als
  höchste Präzedenz-Schicht — wenn Ad-hoc-Override (CI/One-off) real gebraucht wird.
  Erfordert `render_skill`-Signatur-Umbau (Options-Struct) — eigener Spec/Plan.
- **Per-Skill-Vars** (`.lean-ctx/lean-md/skills/<name>/vars.toml`) zusätzlich zum projekt-
  globalen File, falls Skills divergierende Werte brauchen. Aktuell projekt-global (flach).
- **Nicht-String-Werte / TOML-Tabellen** — aktuell flache String-Keys (YAGNI).

## Verifizierte IST-Fakten (Code-Anker)

- **`src/args.rs` `DirectiveArgs`**: unterstützt benannte Args mit double-quoted Werten samt
  Leerzeichen (Test `parses_double_quoted_value_with_spaces`: `old="foo bar"` → `"foo bar"`).
  → `@var test_cmd default="cargo nextest run" desc="…"` parst zu
  `positional(0)="test_cmd"`, `get("default")="cargo nextest run"`, `get("desc")="…"`. ✅
- **`src/parser/inline.rs`**: Inline-Direktiven sind `{{ name args }}` (Trigger `{{ `);
  `{{ var test_cmd }}` → `LmdInline{name:"var", args:"test_cmd"}` → Bridge-Dispatch
  mit `DirectiveArgs::parse("test_cmd")` (nur positional → Lookup-Modus). Registrierte
  Direktive schlägt Value-Tier (`inline_known_directive_still_dispatches`, engine.rs:802). ✅
- **`src/bridges/env.rs`** ist die Vorlage für `@var` (trivialer Lookup-Bridge,
  `name()`/`execute()`, registriert in `default_registry()`).
- **`src/skills.rs::render_skill`**: parst Header, baut `EngineContext::new(header, jail_root)`,
  ruft `capture_phase_bodies` + isoliert die Phase. Pre-Pass-Hook landet zwischen
  `EngineContext::new` und dem Phasen-Render.
- **`src/pathx.rs::jail_path(candidate, jail_root)`**: canonicalisiert den nächsten
  existierenden Vorfahren (resolviert `..`+Symlinks) vor dem Prefix-Check → der `vars.toml`-
  Read ist jailed wie der D7-Overlay (Spec #1, Task 4).
- **`.gitignore`** enthält bereits `.lean-ctx/lean-md/` (Spec #1, Task 8 `f6b8830`) →
  `vars.toml` ist automatisch gitignored, aber zur Render-Zeit voll lesbar.

## File-Zusammenfassung

**Neu:** `src/bridges/var.rs`, `src/skill_vars.rs`.
**Geändert:** `src/engine.rs` (`vars`-Feld + Accessoren), `src/skills.rs` (Pre-Pass),
`src/bridges/mod.rs` (`@var` registrieren), `src/bin/lean_md.rs` (`skill vars --init`),
`src/lib.rs` (`pub mod skill_vars;`),
`content/skills/lmd-test-driven-development/body.lmd.md` (`@var` + `{{ var test_cmd }}`).
**Lokal (gitignored):** `.lean-ctx/lean-md/vars.toml`.
