@lean-md
consumer: ai

@var test_cmd default="cargo nextest run"
@import .lean-ctx/lean-md/plan-recipes /

# `.lmd.md`-Rohtext-Zugriff härten — `lean-md source` + Skill-/Seed-Doku (Implementierungsplan)

**Spec:** `docs/lean-md/specs/2026-07-04-lmd-md-source-read-hardening-design.md`
**Datum:** 2026-07-04

## Goal

Ein `/lmd-writing-plans`-Lauf, der `.lmd.md`-**Seeds** editiert, braucht deren
**Rohtext** für exakte Edit-Anker. Heute existiert kein Pfad dorthin: `ctx_read`
rendert jede `.lmd.md` in **allen** Modi (auch `raw`) durch den lean-md-Renderer →
`@import`-NotFound-Kaskade, `@phase`-Isolation kollabiert, `@define`-Makros werden
konsumiert (Datei „wirkt leer"). Native `cat`/`Read` sind per Hard-Rule verboten.

Dieser Plan schließt die Lücke (Fall B) mit einem neuen CLI-Verb `lean-md source
<file.lmd.md>` (byte-identischer Rohtext, kein Rendering) und dokumentiert die
Zugriffs-Map dort, wo Agenten sie brauchen (geteiltes `hard-rules`-Seed +
`plan-format`-Phase des writing-plans-Skills), sodass kein Rückfall auf
`mode=full`/`mode=raw`/`cat` mehr passiert. Anschließend werden die dann redundanten
Workaround-Prosa-Blöcke in der Projekt-Doku entschlackt (DRY, kein Drift).

## Architecture

- **`src/bin/lean_md.rs`** — neuer Match-Arm `"source" => cmd_source(&args[1..])`
  neben `render|check|mcp|skill` (fn `main`), neue fn `cmd_source` (reine Funktion
  von Dateiinhalt: `load_file` + `print!` → byte-identisch, #498), erweiterte
  Usage-Zeile. Bewusst ein **eigenes Verb** (nicht `render --raw-source`):
  semantisch das Gegenteil von `render` (Quelle vs. gerendert), konsistent mit dem
  Verb-Stil `check`/`skill`.
- **`tests/source_verb.rs`** (neu) — Integrations-Test, spawnt die Binary via
  `env!("CARGO_BIN_EXE_lean-md")`: `source` ist byte-identisch zur Quelle;
  Gegen-Assert `render` derselben Datei konsumiert die Makros.
- **`content/core/hard-rules.lmd.md`** — geteilter `.lmd.md`-Zugriffs-Fakt. Wird
  via `@include` in **jeden** Skill gezogen (brainstorm/writing-plans/writing-skills/
  tdd zugleich). Über `include_str!` als `HARD_RULES`-Const in `src/fragments.rs`
  gebunden → **Seed-Datei ändern genügt**, die Const ist compile-zeitlich
  byte-synchron (Fragment-Konsistenz-Gate `builtin_fragments_match_seed_files_on_disk`
  bleibt grün).
- **`content/skills/lmd-writing-plans/body.lmd.md`** — Point-of-use-Block in
  `@phase "plan-format"` (via `include_str!` als Skill-Body embedded → auto-synchron).
- **`.claude/rules/subagent-multi-agent.md`** + **`CLAUDE.md`** — plain Markdown
  (kein Render-Problem, `ctx_read` bleibt korrekt): Workaround-Prosa eindampfen,
  auf `lean-md source` + `hard-rules` verweisen statt zu duplizieren.

**Dogfooding-Reihenfolge (bindend):** Task 1 (`source`-Verb) MUSS zuerst laufen —
Task 2/3 lesen den Rohtext ihrer `.lmd.md`-Editierziele **mit dem neuen Verb**
(`lean-md source <file>`), weil `ctx_read` sie rendert. Task 4 (Doku-Cleanup) läuft
**nach** 1–3 (Spec: „nach Umsetzung der Komponenten 1–3").

## Global Constraints (jede Task inkludiert dies implizit)

- **Tests immer** `cargo nextest run` — **nie** `cargo test`.
- **Shell — kein** `&&`/`||`/`;`-Chaining: jede Anweisung ist eine eigene Invocation.
  Conditional-Gates in separate Schritte mit explizitem „Expected:" auflösen.
- **Vor jedem `git add` einer `.rs`-Datei:** `cargo fmt` (Standalone-Crate,
  `Cargo.toml` + `src/` im Repo-Root). Für reine `content/*.lmd.md`-Seeds und
  `.md`-Doku (Markdown) ist kein fmt nötig.
- **`cargo clippy --all-targets -- -D warnings`** muss sauber bleiben.
- **#498-Determinismus:** `source` ist eine reine Funktion des Dateiinhalts
  (byte-stabil); kein Timestamp/Counter/Random im Output. `include_str!`-Seeds
  bleiben byte-identisch (Fragment-Gate grün).
- **Sprache:** Chat/Plan/Spec = Deutsch mit Umlauten; **aller gewobene Content
  (Seed-/Skill-Body-Prosa) und jeder Code-Kommentar = Englisch.**
- **Rendern der lmd-Skills in diesem Dev-Repo** läuft direkt über die CLI:
  `cargo run -q --bin lean-md -- render …` — **nicht** über `ctx_md_render`/MCP.
- **`.lmd.md`-Rohtext lesen:** ab Task 1 `cargo run -q --bin lean-md -- source
  <file>` (raw bytes, umgeht den Renderer) — **nie** `ctx_read`/`raw`/`cat` für
  `.lmd.md`-**Quelle** (beide rendern sie). `.md`-Dateien (Specs, Projekt-Doku)
  sind plain Markdown → `ctx_read` bleibt dort korrekt.

---

@phase "task-1"

## Task 1: CLI-Verb `lean-md source <file.lmd.md>` — Rohtext-Pfad (Spec §Design.1)

**Art:** Code-Task, TDD. Schließt die Fall-B-Lücke; **MUSS vor Task 2/3 landen**
(die dogfooden `source`). **Files:** `tests/source_verb.rs` (neu),
`src/bin/lean_md.rs`. **Content/Code = English.**

**Interfaces (verbatim, strikt):**
- Verb: `lean-md source <file.lmd.md>` → schreibt die Datei **byte-identisch** auf
  stdout, **kein** Rendering, **kein** `@import`/`@define`/`@phase`-Processing,
  **kein** `--consumer`/`--crp`.
- Fehlerfälle: fehlendes `<file>`-Argument → `exit:1` mit Hinweis; nicht existierende
  Datei → `exit:1` mit Pfad im Fehler (via `load_file`, wie `cmd_render`/`cmd_check`).

**Schritt 1 — Integrations-Test zuerst schreiben (RED).** Neue Datei
`tests/source_verb.rs` anlegen (new code, verbatim):

    //! `lean-md source <file.lmd.md>` — raw file bytes, NO rendering (Fall B: edit
    //! anchors for `.lmd.md` seeds). Contrast: `render` of the same file consumes
    //! the macros. Proves the raw-source path bypasses the renderer (spec §Design.1).
    use std::process::Command;

    const BIN: &str = env!("CARGO_BIN_EXE_lean-md");

    /// A fixture whose macros the renderer WOULD consume: a failing `@import`
    /// (NotFound cascade) plus a local `@define`/`@call` pair. `source` must return
    /// every byte verbatim; `render` must not.
    const FIXTURE: &str =
        "# Fixture\n@import ./does-not-exist /\n@define greet(name) = Hello name\n@call greet(\"world\")\ntail line\n";

    fn write_fixture() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("lmd_source_verb_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fixture.lmd.md");
        std::fs::write(&path, FIXTURE).unwrap();
        path
    }

    #[test]
    fn source_emits_raw_bytes_verbatim() {
        let path = write_fixture();
        let out = Command::new(BIN)
            .arg("source")
            .arg(&path)
            .output()
            .expect("run lean-md source");
        assert!(out.status.success(), "source must exit 0");
        let stdout = String::from_utf8(out.stdout).unwrap();
        // Byte-identical to the on-disk source — no rendering, no NotFound comment.
        assert_eq!(stdout, FIXTURE, "source must be byte-identical to the file");
        assert!(
            stdout.contains("@define greet(name)"),
            "raw source must keep the @define directive verbatim"
        );
        assert!(
            !stdout.contains("NotFound"),
            "source must not render the @import (no NotFound comment)"
        );
    }

    #[test]
    fn render_consumes_macros_source_does_not() {
        let path = write_fixture();
        let source = Command::new(BIN)
            .arg("source")
            .arg(&path)
            .output()
            .expect("run lean-md source");
        let rendered = Command::new(BIN)
            .arg("render")
            .arg(&path)
            .output()
            .expect("run lean-md render");
        let source = String::from_utf8(source.stdout).unwrap();
        let rendered = String::from_utf8(rendered.stdout).unwrap();
        // Counter-assert: the renderer consumes the @define macro; raw source keeps it.
        assert_ne!(rendered, source, "render must differ from raw source");
        assert!(
            !rendered.contains("@define greet(name)"),
            "render must consume the @define macro"
        );
    }

    #[test]
    fn source_missing_arg_fails() {
        let out = Command::new(BIN)
            .arg("source")
            .output()
            .expect("run lean-md source");
        assert!(!out.status.success(), "missing <file> must exit non-zero");
    }

**Schritt 2 — Test laufen, Fehlschlag bestätigen (RED).**

    @call test("source")

**Expected:** FAIL/Compile-Fehler bzw. `exit:1`-Panics — das `source`-Verb existiert
noch nicht (fällt in den `_ =>`-Usage-Arm, exit 1, stdout leer → `assert_eq!` bricht).

**Schritt 3 — `cmd_source` implementieren.** In `src/bin/lean_md.rs` **nach** der fn
`cmd_check` (Anker: die Funktion, die mit `println!("{}", do_check(&source));` endet)
diese neue fn einfügen (new code, verbatim):

    fn cmd_source(rest: &[String]) {
        // Raw source bytes — bypasses the renderer entirely (no @import/@define/
        // @phase processing, no --consumer/--crp). Fall B: exact edit anchors for
        // `.lmd.md` seeds. Pure function of file content → byte-stable (#498).
        let Some(file) = rest.iter().find(|a| !a.starts_with('-')) else {
            eprintln!("lean-md source: missing <file.lmd.md>");
            std::process::exit(1);
        };
        let (source, _jail) = load_file(file);
        print!("{source}");
    }

**Schritt 4 — Match-Arm + Usage-Zeile ergänzen.** In der fn `main` den `match action`
erweitern. Ersetze (Anker, verbatim):

    "skill" => cmd_skill(&args[1..]),
        _ => {
            eprintln!(
                "Usage: lean-md <render|check|mcp|skill> [args]\n\
                 \n  render <file.lmd.md|--skill NAME [--phase P | --companion C]> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]\
                 \n  check  <file.lmd.md>\
                 \n  mcp                   (stdio JSON-RPC 2.0 MCP server)\
                 \n  skill  <install|remove> <name> [--global|--local]\
                 \n  skill  vars --init [name]"
            );
            std::process::exit(1);
        }

durch:

    "skill" => cmd_skill(&args[1..]),
        "source" => cmd_source(&args[1..]),
        _ => {
            eprintln!(
                "Usage: lean-md <render|check|mcp|skill|source> [args]\n\
                 \n  render <file.lmd.md|--skill NAME [--phase P | --companion C]> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]\
                 \n  check  <file.lmd.md>\
                 \n  source <file.lmd.md>  (raw file bytes, no rendering — for edit anchors)\
                 \n  mcp                   (stdio JSON-RPC 2.0 MCP server)\
                 \n  skill  <install|remove> <name> [--global|--local]\
                 \n  skill  vars --init [name]"
            );
            std::process::exit(1);
        }

**Schritt 5 — `cargo fmt` (vor `git add` der `.rs`-Dateien).**

    `cargo fmt`

**Schritt 6 — Test grün.**

    @call test("source")

**Expected:** PASS — alle drei Tests (`source_emits_raw_bytes_verbatim`,
`render_consumes_macros_source_does_not`, `source_missing_arg_fails`).

**Schritt 7 — Manuelle Dogfood-Probe (Byte-Stabilität, #498).** Das Verb an einer
echten `.lmd.md` mit Direktiven prüfen — zweimal, byte-identisch:

    `cargo run -q --bin lean-md -- source content/skills/lmd-writing-plans/body.lmd.md`

**Expected:** die Ausgabe zeigt die rohen `@phase "…"`/`@phase-end`-Marker und
`@include`/`@call`-Zeilen **unverändert** (kein `next: render phase`, kein
`PHASE_ABORTED`, kein NotFound-Kommentar); zweiter identischer Aufruf → byte-gleich.

**Schritt 8 — Diff sichten + Commit.**

    @call verify("src/bin/lean_md.rs tests/source_verb.rs")

    @call commit("src/bin/lean_md.rs tests/source_verb.rs", "feat(lean-md): source verb — raw .lmd.md bytes, no rendering (Fall B edit anchors)")

**Deliverable:** `lean-md source <file>` liefert byte-identischen Rohtext; Gegen-Assert
belegt, dass `render` die Makros konsumiert. Grüne Suite.

@phase-end

---

@phase "task-2"

## Task 2: geteilter `.lmd.md`-Zugriffs-Fakt in `hard-rules.lmd.md` (Spec §Design.2)

**Art:** Content-Härtung → Render-Gate + Fragment-Gate (kein neuer Test-Code).
**Files:** `content/core/hard-rules.lmd.md`. **Content = English.** **Dogfood Task 1:**
das Editierziel ist eine `.lmd.md` — Rohtext via `lean-md source` lesen, **nicht**
`ctx_read`.

**Schritt 1 — Rohtext des Seeds via neuem Verb lesen (Anker setzen).**

    `cargo run -q --bin lean-md -- source content/core/hard-rules.lmd.md`

**Expected (Ist-Stand, byte-genauer Anker):**

    # Hard Rules (lmd built-in)
    - Never native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
    - All other I/O + code-intel runs through lean-ctx tools — see `tooling/mcp-tools`;
      language-specific symbol/edit/reformat conventions live in `lang/<lang>` (e.g. `lang/rust`).

**Schritt 2 — neuen Bullet-Block anhängen.** In `content/core/hard-rules.lmd.md`
**nach** der letzten Zeile (Anker: die Zeile, die auf
`` conventions live in `lang/<lang>` (e.g. `lang/rust`). `` endet) diesen Block
anfügen (new content, verbatim — English, voller Block mit Ursache + Rohtext-Pfad,
damit kein Nachschlagen nötig ist):

    - A `.lmd.md` is a **rendered artifact, not a source file**. `@read`/ctx_read
      (any mode, `raw` included) and every `render` path RENDER it → `@import` NotFound
      cascade, `@phase` isolation collapse, `@define` macros consumed (the file looks
      "empty"). Access it with lean-md renderer means, per intent:
      - a task/phase brief                                   → `render --phase <p>`
      - the macro API index                                 → `render --signatures`
      - the raw source (copy shape / set exact edit anchors) → `lean-md source <file>`
      Never native cat/Read and never ctx_read for `.lmd.md` source — both render it.

**Schritt 3 — Fragment-Gate grün (Seed == built-in Const).** `include_str!` zieht das
Seed compile-zeitlich in `HARD_RULES` ein → `cargo nextest run` rekompiliert, die
Const ist byte-synchron:

    @call test("hard_rules")

**Expected:** PASS — `builtin_fragments_match_seed_files_on_disk` (built-in ==
on-disk-Seed, #498), `hard_rules_slim` (die entfernten Prosa-Marker
`"prefer symbol-aware"`/`"reformat before"` bleiben abwesend; `tooling/mcp-tools`
+ `lang/` bleiben präsent), `hard_rules_has_no_stale_backings` (kein
`serena`/`jetbrains`) — der neue Block verletzt keine Assertion.

**Schritt 4 — Render-Gate: der Fakt erscheint via `@include` in einem Skill.**

    `cargo run -q --bin lean-md -- render --skill lmd-writing-plans --phase self-review --consumer=ai`

**Expected:** der gerenderte `# Hard Rules (lmd built-in)`-Block (via `@include`
im auto-vorangestellten Dispatch-Contract) enthält jetzt die Zeile
`the raw source (copy shape / set exact edit anchors) → \`lean-md source <file>\``.

**Schritt 5 — Commit** (reines Markdown-Seed → kein `cargo fmt`).

    @call commit("content/core/hard-rules.lmd.md", "docs(hard-rules): shared .lmd.md access map — render/signatures/source per intent, never ctx_read source")

**Deliverable:** geteilter Rohtext-/Zugriffs-Fakt in jedem Skill sichtbar; Fragment-
und Render-Gate grün.

@phase-end

---

@phase "task-3"

## Task 3: `plan-format`-Point-of-use-Block im writing-plans-Body (Spec §Design.3)

**Art:** Content-Härtung → Render-Gate (kein neuer Test-Code). **Files:**
`content/skills/lmd-writing-plans/body.lmd.md`. **Content = English.** **Dogfood
Task 1:** der Skill-Body ist eine `.lmd.md` mit `@phase`-Markern — Rohtext **nur** via
`lean-md source` erreichbar (`ctx_read`/`raw` würde ihn rendern → `PHASE_ABORTED`,
Marker verschwinden).

**Schritt 1 — Rohtext des Skill-Bodys via neuem Verb lesen (Anker finden).**

    `cargo run -q --bin lean-md -- source content/skills/lmd-writing-plans/body.lmd.md`

**Expected:** die rohe `@phase "plan-format"`-Sektion ist sichtbar. Der Einfüge-Anker
ist der Boilerplate-Absatz dieser Phase, der auf die `--signatures`-Discovery-Zeile
endet — im gerenderten Text lautet er:

> **Boilerplate** (TDD cycle, commit, test-run) → `@call <recipe>(...)`; … To
> discover which recipes exist, read the macro API index instead of the whole
> library: `lean-md render .lean-ctx/lean-md/plan-recipes.lmd.md --signatures`.

**Schritt 2 — Point-of-use-Block einfügen.** In `content/skills/lmd-writing-plans/body.lmd.md`
innerhalb `@phase "plan-format"`, **direkt nach** dem Boilerplate-/`--signatures`-Absatz
(vor dem `## No Placeholders`-Abschnitt), diesen Block einfügen (new content, verbatim —
English):

    **Reading the `.lmd.md` sources while authoring (see Hard Rules):** a plan,
    template, recipe library or seed is a rendered artifact — `@read mode=full`|`auto`
    AND `mode=raw` both render it (macros vanish, file looks empty). Access map:
    - recipe macro API (`plan-recipes.lmd.md`)              → `render … --signatures`
    - an existing plan / template phase brief               → `render … --phase <p>`
    - raw source of a seed/template you must EDIT (anchors) → `lean-md source <file>`

**Schritt 3 — Render-Gate: der Block erscheint in der `plan-format`-Phase.**

    `cargo run -q --bin lean-md -- render --skill lmd-writing-plans --phase plan-format --consumer=ai`

**Expected:** die Ausgabe enthält jetzt den Block mit
`raw source of a seed/template you must EDIT (anchors) → \`lean-md source <file>\``;
zweiter identischer Aufruf → byte-identisch (#498). Der Skill-Body ist via
`include_str!` embedded → Const auto-synchron, kein separater Sync-Schritt.

**Schritt 4 — Phase-Isolation prüfen (kein Cross-Task-Leck).** Eine Nachbar-Phase
rendern und sicherstellen, dass der neue Block **nicht** dort erscheint:

    `cargo run -q --bin lean-md -- render --skill lmd-writing-plans --phase write-plan --consumer=ai`

**Expected:** `write-plan` enthält den `lean-md source`-Block **nicht** (Block ist
`plan-format`-lokal).

**Schritt 5 — Volle Suite grün (Skill-Render-Gates).**

    @call test("")

**Expected:** PASS — inkl. der Skill-Render-Determinismus-Gates in
`src/bin/lean_md.rs` (`skill_render_is_byte_stable_and_isolated` etc.).

**Schritt 6 — Commit.**

    @call commit("content/skills/lmd-writing-plans/body.lmd.md", "docs(lmd-writing-plans): plan-format point-of-use — .lmd.md source access map (signatures/phase/source)")

**Deliverable:** die `plan-format`-Phase nennt den Rohtext-Pfad am Point-of-use;
Render-Gate + Phase-Isolation grün.

@phase-end

---

@phase "task-4"

## Task 4: Doku-Cleanup — Workaround-Prosa eindampfen (Spec §Design.4)

**Art:** Doku-Task, **plain Markdown** (`.md` → `ctx_read` ist korrekt, KEIN
`lean-md source` nötig). Verifikation = manuelle Sichtprüfung (Spec). Läuft **nach**
Task 1–3, weil `source` + Seed-Härtung die lange Prosa erst redundant machen.
**Files:** `.claude/rules/subagent-multi-agent.md`, `CLAUDE.md`. **Content = Deutsch**
(diese Dateien sind projektintern deutschsprachig — bestehenden Ton beibehalten).

**Leitprinzip:** die **normative** Regel lebt künftig einmal im Seed (`hard-rules`,
via Render sichtbar) + im CLI-Verb; die Projekt-Doku **verweist** darauf statt zu
duplizieren. **Nicht** löschen, was weiter gilt (Controller rendert Task-Briefs via
`--phase`, `raw=true` gegen Doppel-Kompression).

**Schritt 1 — `CLAUDE.md`-Block „Rendering lmd-skills" um Fall B ergänzen.** Ersetze
in `CLAUDE.md` (Anker, verbatim — der bestehende Block):

    - **Rendering lmd-skills (this dev-repo)**: the `SKILL.md` stubs point at the MCP
      tool `ctx_md_render`, which is **not registered** in this repo's lean-ctx
      instance — that call fails. Render phases **directly via the CLI**; do NOT probe
      MCP / `ctx_call` first:
      `cargo run -q --bin lean-md -- render --skill <skill> --phase <phase> --consumer=ai`
      (companion instead of phase: `--companion <name>`). **No release build** —
      `cargo run` suffices (cached after the first compile).

durch (Fall A bleibt Kern; Fall B verweist auf `source` statt `ctx_read`/`raw`):

    - **Rendering lmd-skills (this dev-repo)**: the `SKILL.md` stubs point at the MCP
      tool `ctx_md_render`, which is **not registered** in this repo's lean-ctx
      instance — that call fails. Render phases **directly via the CLI**; do NOT probe
      MCP / `ctx_call` first:
      `cargo run -q --bin lean-md -- render --skill <skill> --phase <phase> --consumer=ai`
      (companion instead of phase: `--companion <name>`). **No release build** —
      `cargo run` suffices (cached after the first compile).
    - **Reading `.lmd.md` raw source (Fall B — edit anchors):**
      `cargo run -q --bin lean-md -- source <file>` — raw bytes, bypasses the
      renderer. **Never** `ctx_read`/`raw`/`cat` for `.lmd.md` *source* (all render
      it → `@import` NotFound, `@define`/`@phase` consumed). The normative rule lives
      in the `hard-rules` seed; this is just the dev-repo CLI form.

**Schritt 2 — `subagent-multi-agent.md` eindampfen.** Datei frisch lesen (plain `.md`,
`ctx_read` korrekt), Abschnitt *„Plan brief = CLI phase render"*. Der Fall-A-Kern
bleibt: Brief = `render --phase`, `raw=true` **mandatory** (gegen Doppel-Kompression,
sonst mangelt der zweite Kompressor den zu schreibenden Code). **Entfernen/kürzen:**
die lange `String`→`str`-Mangling-Beispielliste **und** die ausführliche
*„Never `ctx_read` a plan `.lmd.md`"*-Rohtext-Begründung — beide sind jetzt durch
`lean-md source` + das `hard-rules`-Seed abgedeckt. Ersetze den ausufernden
`raw=true`-Rationale-Absatz (Anker: beginnt mit
`**`raw=true` is mandatory here — do not stack a second compressor.**`) durch die
gestraffte Fassung:

    **`raw=true` is mandatory here — do not stack a second compressor.** lean-md's
    render is already the terse, byte-stable (#498) artifact; piping it through the
    dense `ctx_shell` default re-compresses and mangles the code an implementer must
    write verbatim. This is the `AGENTS.md` "never `ctx_shell raw=true` unless
    compression is provably wrong" exception — for code-to-write it *is* provably
    wrong.

und ersetze den *„Never `ctx_read` a plan `.lmd.md`"*-Absatz (Anker: beginnt mit
`**Never `ctx_read` a plan `.lmd.md` — render it.**`) durch:

    **Never `ctx_read` a plan `.lmd.md` — render it.** It's a rendered artifact:
    `mode=full`/`auto` renders it whole-doc (→ `@import` NotFound cascade, lost
    `@phase` isolation), `mode=raw` still renders it (macros consumed). Controller
    orientation = render each `--phase`. To read the **raw source** of a `.lmd.md`
    you must EDIT (exact edit anchors, Fall B), use `lean-md source <file>` — the
    normative `.lmd.md`-access rule lives in the `hard-rules` seed (see it via any
    skill render); this file no longer restates it.

**Schritt 3 — Konsistenz-Grep: keine Rohtext-Workaround-Prosa mehr.** Prüfen, dass die
alten `ctx_read`/`raw`-Umweg-Beschreibungen weg sind und `lean-md source` referenziert
wird:

    Run: `@search "lean-md source"` over `CLAUDE.md` and `.claude/rules/subagent-multi-agent.md`
    — Expected: hits in **both** files (Fall-B-Verweis vorhanden).

    Run: `@search "mode=raw"` over `.claude/rules/subagent-multi-agent.md`
    — Expected: höchstens der eine gestraffte „mode=raw still renders it"-Verweis;
    **keine** lange Rohtext-Workaround-Prosa mehr.

**Schritt 4 — Sichtprüfung (Spec-Verifikation).** Beide Dateien lesen (`ctx_read`,
plain `.md`): der Fall-A-Render-Pfad (Brief = `render --phase`, `raw=true`) ist
**erhalten**; die Rohtext-Erklärung ist durch den `lean-md source`- + `hard-rules`-
Verweis **ersetzt** (nicht dupliziert).

**Schritt 5 — Commit** (plain Markdown → kein `cargo fmt`).

    @call commit(".claude/rules/subagent-multi-agent.md CLAUDE.md", "docs: point .lmd.md raw-source access at `lean-md source` + hard-rules seed; drop redundant workaround prose")

**Deliverable:** keine `.lmd.md`-Rohtext-Workaround-Prosa mehr in `CLAUDE.md` /
`subagent-multi-agent.md`; Fall-A bleibt; beide verweisen auf `lean-md source` bzw.
`hard-rules` statt zu duplizieren.

@phase-end

---

@phase "task-5"

## Task 5: Full-Gate + Reference-Closure

**Art:** Verifikations-Task (kein neuer Code). Stellt die Gesamtänderung grün und die
Bindung konsistent.

**Schritt 1 — komplette Testsuite.**

    @call test("")

**Expected:** alle Tests grün, inkl. `tests/source_verb.rs` (Task 1) und der
Fragment-/Skill-Render-Gates (Task 2/3).

**Schritt 2 — Clippy gesamt.**

    `cargo clippy --all-targets -- -D warnings`

**Expected:** keine Warnings.

**Schritt 3 — Post-Change-Gate (Public-CLI-Fläche + Multi-File-Änderung).**

    @call review_change()

**Schritt 4 — Render-Determinismus final (#498).** Die zwei gehärteten Content-Pfade
je zweimal rendern, Ausgaben byte-identisch:

    `cargo run -q --bin lean-md -- render --skill lmd-writing-plans --phase plan-format --consumer=ai`
    `cargo run -q --bin lean-md -- render --skill lmd-writing-plans --phase self-review --consumer=ai`

**Expected:** `plan-format` enthält den `lean-md source`-Block; `self-review` (Hard
Rules via `@include`) enthält die `lean-md source <file>`-Zeile; jeweils byte-stabil.

**Schritt 5 — durable Abschluss-Notiz.**

    @call remember_decision("lmd .lmd.md source-read hardening done: new `lean-md source <file>` verb (raw bytes, no render, tests/source_verb.rs); shared access map in hard-rules seed + writing-plans plan-format phase; CLAUDE.md + subagent-multi-agent.md point at `lean-md source`/`hard-rules` instead of duplicating workaround prose. See plan Task 1-5.")

**Deliverable:** grüne Full-Gate, konsistente Zugriffs-Map, kein offener Rohtext-
Workaround.

@phase-end
