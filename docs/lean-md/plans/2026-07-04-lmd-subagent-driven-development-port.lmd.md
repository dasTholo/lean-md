@lean-md 0.4
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings"
@import .lean-ctx/lean-md/plan-recipes /

@define gate(paths)
<!-- Pre-commit quality bar: reformat the changed paths, clippy (-D warnings), full nextest suite -->
1. Run: `@reformat {{ paths }}` — rustfmt via ctx_refactor before staging (project rule: fmt before git add).
2. Run: {{ var lint_cmd }} — Expected: no warnings.
3. Run: {{ var test_cmd }} — Expected: PASS (full suite).
@define-end

@define render_check(skill, phase)
<!-- Render one skill phase via the CLI; assert non-empty + byte-stable (#498) -->
Run: cargo run -q --bin lean-md -- render --skill {{ skill }} --phase {{ phase }} --consumer=ai
— Expected: non-empty, no eval err, byte-identical across two runs.
@define-end

# lmd-subagent-driven-development — Port-Implementierungsplan

## Goal

Den superpowers-Skill `subagent-driven-development` **ohne Funktionsverlust** als nativen
lmd-Skill portieren: ein `.lmd.md`-Plan wird ausgeführt, pro Task ein frischer
Implementer-Subagent, danach Zwei-Verdikt-Review, am Ende Whole-Branch-Review — alle
Handoffs über lean-ctx-Memory/Coordination statt superpowers-Bash-Skripte/Datei-Artefakte.
Quelle: `docs/lean-md/specs/2026-07-04-lmd-subagent-driven-development-port-design.md`.

Der Skill selbst ist schlanke Koordinations-Prosa (Body, 6 Phasen); die eigentlichen
Subagent-Instruktionen leben isoliert in drei Companions. Zusätzlich entstehen die
Renderer-/Directive-Enabler, die der Body braucht: zwei neue Directive-Bridges
(`@checkpoint`/`@compress`), drei neue `on-complete`-Sinks (`return`/`handoff`/`sync`) plus
Sink-Rename `checkpoint→compress`, drei Plan-Recipes und die import-unabhängige
Phase-Outline-Capability (`render --list-phases`).

## Architecture

**Datei-Layout — die Seed-Dateien sind BEREITS materialisiert** (mit diesem Plan committet),
weil `include_str!` sie zur Compile-Zeit am Zielort braucht und der Content die eigentliche
Portier-Prosa ist (bereits plan-reviewt). Task 6 **verdrahtet** sie, er erstellt den Content
nicht:

    content/skills/lmd-subagent-driven-development/
      SKILL.md                      # Delegation-Stub (Phasen-Index + Companions)
      body.lmd.md                   # 6 Phasen: orient…handoff
      companions/
        implementer.lmd.md          # Dispatch-Brief (Contract auto-prepended)
        task-reviewer.lmd.md        # Zwei-Verdikt-Review-Brief
        code-reviewer.lmd.md        # Whole-Branch-Final-Review-Brief

**Warum vor-materialisiert statt inline im Brief:** ein `.lmd.md`-Plan wird selbst gerendert;
zeilenanfängliche `@phase`/`@define`/`@lean-md`-Direktiven im Brief-Content werden vom Renderer
(`render_with_phases`, `line.trim_start()` — kein Fence-/Indent-Guard) als **echt** interpretiert
→ der Brief bricht ab. Direktiven-tragender Seed-Content kann daher nicht inline gezeigt werden;
er liegt am Zielort und wird bei Bedarf raw referenziert (`lean-md source`). Ebenso sind die drei
Plan-Recipes bereits an `content/templates/plan-recipes.lmd.md` angehängt (Append an eine
bestehende Datei; Task 5 verifiziert sie nur).

**Verdrahtungspunkte je Skill (alle in Task 6 abgedeckt):**
1. `content/skills/<name>/` — Dateien (bereits da, s. o.).
2. `src/skills.rs` — `include_str!`-Consts (Body + je Companion), `SKILLS`-Row (Body),
   `COMPANIONS`-Rows.
3. `src/skill_install.rs` — `SKILL_MD`-Const + `INSTALLABLE_SKILLS`-Row (sonst NICHT
   installierbar; SDD hat keine `ASSETS`).
4. `src/availability.rs` — `COVERAGE`-Rows.

**Fragment-Konsistenz-Gate** (built-in `include_str!` == on-disk Seed) + der `@dispatch`-Contract-
Auto-Prepend (Companions tragen **keinen** Contract-Text) gelten für jeden Seed gleich.

**Bridge-Muster (`src/bridges/*.rs`):** jede neue Bridge ist ein `pub struct XBridge` +
`impl DirectiveBridge`, das `ctx.backend.call("ctx_*", args)` outbound feuert (CLI default /
MCP opt-in), byte-stabile Tool-Text (#498), headless → verworfener `BACKEND_REQUIRED`-Envelope.
Vorlage: `src/bridges/smells.rs`. Registrierung: eine Zeile in `default_registry()`
(`src/bridges/mod.rs`), eine Gloss-Row in `content/gloss/directives.lmd.md`.

**Sink-Muster (`src/phases.rs`):** `on-complete`-Sinks werden in `parse_on_complete` erkannt
und in `fire_action` gefeuert; team-orientierte Sinks laufen über `fire_agent` →
`ctx.backend.call("ctx_agent", …)`. Kein neues `@`-Directive.

**Phase-Outline (`src/phases.rs` + `src/bin/lean_md.rs` + `src/lib.rs`):** ein geteilter,
geordneter Phasen-Scanner `iter_phase_blocks` speist sowohl `capture_phase_bodies` als auch die
neue `outline_phases`; das CLI-Flag `--list-phases` ist nur ein Konsument der Lib-Funktion.

## Global Constraints

- **Tests**: immer `cargo nextest run`, nie `cargo test`.
- **Shell**: kein `&&`/`||`/`;`-Chaining — jeder Befehl ist eine eigene Invocation.
- **Pre-commit** (pro geänderter Datei): rustfmt (via `@reformat`/`cargo fmt`), dann `git add`.
- **#498 Byte-Stabilität**: alle Tool-/Render-Outputs sind deterministische Funktionen von
  (Content, Mode, CRP, Task) — keine Timestamps/Counter. Bestehende Tests bleiben grün.
- **Sprache**: gewobener Seed-Content (Body/Companions) + Rust-Code/-Kommentare = **Englisch**;
  Plan-/Task-Prosa = Deutsch.
- **Render dieses Plans**: pro Task via `lean-md render <plan>.lmd.md --phase task-N`.
- **Prerequisites (extern, NICHT in diesem Plan)**:
  - **Bug-1-Fix** (quote/komma-bewusster `@call`-Argument-Split, `src/macros.rs`) muss vor
    **Task 5** landen — `task_return(...)` ist ein quoted Ein-Arg-Recipe mit Binnen-Kommas/
    Semikolons. Geliefert von
    `docs/lean-md/plans/2026-07-04-lmd-renderer-prereq-bug1-bug3.lmd.md` (Task 1).
  - **`crp: compact`** im `plan-template` (Terseness-Deliverable) ist bereits im Template
    verankert; **Task 6**s `dispatch_threads_crp_compact_into_contract` verifiziert nur die
    bestehende crp-Threading-Mechanik für den `compact`-Wert (kein Engine-Change nötig).
- **Task-abhängigkeiten**: T2→T1; T5→T3,T4 (+Bug-1 extern); T6→T2,T3,T4,T5. T1/T3/T4 sind
  landing-unabhängig.
- **Seed-Verdrahtung statt -Erstellung**: die 5 Seed-Dateien unter
  `content/skills/lmd-subagent-driven-development/` und die 3 Recipes in `plan-recipes.lmd.md`
  sind bereits materialisiert (s. Architecture). Tasks 5/6 registrieren/verifizieren sie —
  sie tippen keinen Seed-Content ab. Der Brief nennt den Zielpfad; bei Bedarf raw via
  `lean-md source`.

@phase "task-1"

## Task 1 — Phase-Outline Lib-Kern (`iter_phase_blocks` + `outline_phases`)

**Datei:** `src/phases.rs`. **Landing-unabhängig** (Bug-3-immun — kein `@import`-Pfad).

**Interfaces (neu, öffentlich innerhalb der Crate):**

    pub(crate) fn iter_phase_blocks(source: &str) -> Vec<(String, String)>   // (name, raw_body), geordnet
    pub struct PhaseOutline { pub name: String, pub title: String }
    pub fn outline_phases(source: &str) -> Vec<PhaseOutline>

**Anker (bestehend, wird refactored — nicht duplizieren):** `@symbol capture_phase_bodies`
(`src/phases.rs:443`). Der bestehende Scanner erbt die `@phase`/`@phase-end`-Semantik (flat v1,
erster kompletter Block pro Name). `iter_phase_blocks` extrahiert genau diese Scan-Schleife als
**geordnete** `(name, body)`-Liste; `capture_phase_bodies` wird zum dünnen Konsumenten (eine
Quelle der Grenz-Semantik, kein zweiter Parser).

### 1. RED — failing tests

Füge in `src/phases.rs` `#[cfg(test)] mod tests` hinzu (verbatim, neuer Code):

    #[test]
    fn iter_phase_blocks_orders_phases() {
        let src = "@phase \"task-1\"\nA body\n@phase-end\n@phase \"task-2\"\nB body\n@phase-end\n";
        let blocks = iter_phase_blocks(src);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, "task-1");
        assert_eq!(blocks[1].0, "task-2");
        assert!(blocks[0].1.contains("A body"));
        assert!(blocks[1].1.contains("B body"));
    }

    #[test]
    fn outline_derives_title_from_first_heading() {
        let src = "@phase \"task-1\"\n## Task 1 — bug split\nprose\n@phase-end\n";
        let o = outline_phases(src);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].name, "task-1");
        assert_eq!(o[0].title, "Task 1 — bug split");
    }

    #[test]
    fn outline_title_falls_back_when_no_heading() {
        // No heading → first non-empty, non-directive line; else empty.
        let src = "@phase \"task-1\"\n@read a.rs\nfirst prose line\n@phase-end\n@phase \"task-2\"\n@phase-end\n";
        let o = outline_phases(src);
        assert_eq!(o[0].title, "first prose line");
        assert_eq!(o[1].title, "");
    }

    #[test]
    fn outline_is_import_independent() {
        // A source whose @import target is missing still lists every phase — the
        // outline scans @phase markers only, never entering the render/import path.
        let src = "@import .lean-ctx/lean-md/nope /\n@phase \"task-1\"\n## T1\n@phase-end\n";
        let o = outline_phases(src);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].name, "task-1");
    }

    #[test]
    fn outline_is_byte_stable() {
        let src = "@phase \"task-1\"\n## T1\n@phase-end\n@phase \"task-2\"\n## T2\n@phase-end\n";
        assert_eq!(outline_phases(src), outline_phases(src));
    }

`PhaseOutline` braucht `#[derive(Debug, Clone, PartialEq)]` für `assert_eq!`.

Run: `cargo nextest run phases::` — Expected: die 5 neuen Tests FAIL (RED, Symbole existieren
noch nicht); bestehende `capture_phase_bodies`-Tests bleiben grün.

### 2. GREEN — Implementierung

Neuer geteilter Scanner + Outline (verbatim, neuer Code) in `src/phases.rs`:

    /// Ordered scan of every `@phase "name" … @phase-end` block → `(name, raw_body)`.
    /// Single source of the phase-boundary semantics (flat v1: not nested; the first
    /// complete block per name wins). Both `capture_phase_bodies` and `outline_phases`
    /// consume this — no second parser. Byte-stable (#498).
    pub(crate) fn iter_phase_blocks(source: &str) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut open: Option<(String, Vec<&str>)> = None;
        for line in source.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("@phase-end") {
                if let Some((name, lines)) = open.take() {
                    if !out.iter().any(|(n, _)| *n == name) {
                        out.push((name, lines.join("\n")));
                    }
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("@phase") {
                if open.is_none() {
                    open = Some((parse_phase_name(rest), Vec::new()));
                }
                continue;
            }
            if let Some((_, lines)) = open.as_mut() {
                lines.push(line);
            }
        }
        out
    }

    /// A phase's identity for a preflight overview: its name + a human title.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PhaseOutline {
        pub name: String,
        pub title: String,
    }

    /// Import-independent phase index: name + title, no body render, no `@import`.
    /// Title = first `#`/`##` heading in the body (hashes/spaces stripped); else the
    /// first non-empty non-directive line; else empty. Byte-stable (#498).
    pub fn outline_phases(source: &str) -> Vec<PhaseOutline> {
        iter_phase_blocks(source)
            .into_iter()
            .map(|(name, body)| {
                let title = phase_title(&body);
                PhaseOutline { name, title }
            })
            .collect()
    }

    /// Derive a display title from a raw phase body (see `outline_phases`).
    fn phase_title(body: &str) -> String {
        // First markdown heading wins.
        for line in body.lines() {
            let t = line.trim_start();
            if let Some(h) = t.strip_prefix('#') {
                return h.trim_start_matches('#').trim().to_string();
            }
        }
        // Fallback: first non-empty, non-directive line.
        for line in body.lines() {
            let t = line.trim();
            if !t.is_empty() && !t.starts_with('@') {
                return t.to_string();
            }
        }
        String::new()
    }

Baue `capture_phase_bodies` auf `iter_phase_blocks` um (verbatim ersetzt den bestehenden
Schleifen-Body — Anker `@symbol capture_phase_bodies`):

    pub(crate) fn capture_phase_bodies(ctx: &Rc<EngineContext>, body: &str) {
        for (name, raw) in iter_phase_blocks(body) {
            ctx.phase_bodies.borrow_mut().entry(name).or_insert(raw);
        }
    }

Run: `cargo nextest run phases::` — Expected: alle neuen + bestehenden Tests PASS (GREEN),
inkl. `capture_phase_bodies_extracts_raw_body_without_lifecycle`,
`unterminated_phase_is_a_visible_error`, `nested_phase_is_an_error` (Regressionsschutz).

### 3. Verify & Close

@call verify(src/phases.rs)
@call gate(src/phases.rs)
@call commit("src/phases.rs", "feat: shared ordered phase scanner + outline_phases (list-phases lib core)")
@call remember_decision("iter_phase_blocks is the single source of @phase-boundary semantics; capture_phase_bodies + outline_phases both consume it")

@phase-end

@phase "task-2"

## Task 2 — `render --list-phases` CLI + Lib-Re-Export

**Dateien:** `src/bin/lean_md.rs` (`parse_render_flags`, `cmd_render`), `src/lib.rs`
(Re-Export). **Depends:** Task 1.

**Interfaces:**
- `RenderArgs` bekommt ein neues Feld `list_phases: bool`.
- `--list-phases` ist **mutually exclusive** mit `--phase` (Fehler bei beidem).
- `--consumer`/`--crp` werden ignoriert (strukturelle Ausgabe). Läuft auf Plandatei **und**
  `--skill X` (gleicher Source-Load-Pfad).
- Ausgabe: geordnete `name<TAB>title`-Zeilen; phasenlose/leere Source → leerer Output, exit 0.

**Anker (bestehend):** `@symbol parse_render_flags` (`src/bin/lean_md.rs:76`),
`@symbol cmd_render` (`src/bin/lean_md.rs:145`). Der Skill-Zweig in `cmd_render` lädt Skills via
`skill_body(skill)`; der Datei-Zweig via `load_file`.

### 1. RED — failing integration tests

Erstelle `tests/list_phases.rs` (verbatim, neuer Code):

    //! `render --list-phases`: import-independent phase index (name<TAB>title).
    use std::process::Command;

    fn run(args: &[&str], cwd: &std::path::Path) -> (String, String, i32) {
        let out = Command::new(env!("CARGO_BIN_EXE_lean-md"))
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("run lean-md");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code().unwrap_or(-1),
        )
    }

    #[test]
    fn render_list_phases_emits_index() {
        let dir = std::env::temp_dir().join(format!("lmd_listphases_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let plan = dir.join("p.lmd.md");
        std::fs::write(
            &plan,
            "@import .lean-ctx/lean-md/nope /\n@phase \"task-1\"\n## Task 1 — first\n@phase-end\n@phase \"task-2\"\n## Task 2 — second\n@phase-end\n",
        )
        .unwrap();
        let (stdout, _e, code) = run(&["render", plan.to_str().unwrap(), "--list-phases"], &dir);
        assert_eq!(code, 0);
        assert_eq!(stdout, "task-1\tTask 1 — first\ntask-2\tTask 2 — second\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_phases_and_phase_mutually_exclusive() {
        let dir = std::env::temp_dir().join(format!("lmd_listphases_mx_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let plan = dir.join("p.lmd.md");
        std::fs::write(&plan, "@phase \"task-1\"\n## T\n@phase-end\n").unwrap();
        let (_o, stderr, code) = run(
            &["render", plan.to_str().unwrap(), "--list-phases", "--phase", "task-1"],
            &dir,
        );
        assert_ne!(code, 0);
        assert!(stderr.contains("mutually exclusive"), "got: {stderr}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_phases_empty_source_is_empty_exit_zero() {
        let dir = std::env::temp_dir().join(format!("lmd_listphases_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let plan = dir.join("p.lmd.md");
        std::fs::write(&plan, "no phases here\n").unwrap();
        let (stdout, _e, code) = run(&["render", plan.to_str().unwrap(), "--list-phases"], &dir);
        assert_eq!(code, 0);
        assert_eq!(stdout, "");
        let _ = std::fs::remove_dir_all(&dir);
    }

Run: `cargo nextest run --test list_phases` — Expected: alle 3 FAIL (RED, `--list-phases`
unbekannt → als Flag ignoriert, kein Index).

### 2. GREEN — Implementierung

`src/lib.rs`: re-exportiere die Lib-Funktion neben den bestehenden phase-Exports (verbatim):

    pub use phases::{outline_phases, PhaseOutline};

`parse_render_flags` — neues Feld + Flag-Zweig (verbatim ergänzen zu `RenderArgs` /
match-Arm):

    // in struct RenderArgs:
    list_phases: bool,

    // in the match on `arg`, next to "--signatures":
    "--list-phases" => a.list_phases = true,

`cmd_render` — **früher Zweig** vor der Skill-/Datei-Render-Logik (verbatim, neuer Code, direkt
nach `let a = parse_render_flags(rest);`):

    if a.list_phases {
        if a.phase.is_some() {
            eprintln!("lean-md render: --list-phases and --phase are mutually exclusive");
            std::process::exit(1);
        }
        // Load the source the same way the render paths do: skill body or file.
        let source = match a.skill.as_deref() {
            Some(skill) => match lean_md::skills::skill_body(skill) {
                Some(body) => body.to_string(),
                None => {
                    eprintln!("lean-md render: unknown skill '{skill}'");
                    std::process::exit(1);
                }
            },
            None => {
                let Some(file) = a.file.as_deref() else {
                    eprintln!("lean-md render: --list-phases needs <file.lmd.md> or --skill NAME");
                    std::process::exit(1);
                };
                load_file(file)
            }
        };
        for p in lean_md::outline_phases(&source) {
            println!("{}\t{}", p.name, p.title);
        }
        return;
    }

Erweitere den Usage-Text (`main`, Anker `src/bin/lean_md.rs:130`) um `[--list-phases]` in der
`render`-Zeile.

Run: `cargo nextest run --test list_phases` — Expected: alle 3 PASS (GREEN).
Run: `cargo nextest run` — Expected: volle Suite PASS.

### 3. Verify & Close

@call verify(src/bin/lean_md.rs)
@call gate(src/bin/lean_md.rs src/lib.rs tests/list_phases.rs)
@call commit("src/bin/lean_md.rs src/lib.rs tests/list_phases.rs", "feat: render --list-phases import-independent phase index")
@call remember_decision("render --list-phases emits ordered name<TAB>title; import-independent (Bug-3-immune); mutually exclusive with --phase")

@phase-end

@phase "task-3"

## Task 3 — Directive-Bridges `@checkpoint` + `@compress`

**Dateien:** `src/bridges/checkpoint.rs` (neu), `src/bridges/compress.rs` (neu),
`src/bridges/mod.rs` (Registrierung), `content/gloss/directives.lmd.md` (2 Rows).
**Landing-unabhängig.** **Vorlage:** `@symbol SmellsBridge` (`src/bridges/smells.rs`).

**Interfaces:**
- `@checkpoint action=snapshot|log|diff|restore [label=…] [message=…]` → `ctx_checkpoint`
  (Shadow-git, getrennt von User-`.git`). Default action `snapshot`.
- `@compress action=checkpoint` → `ctx_compress` (Session-Kontext-Checkpoint). Default action
  `checkpoint`.

### 1. RED — failing tests

`src/bridges/checkpoint.rs` — `#[cfg(test)] mod tests` (verbatim):

    #[test]
    fn checkpoint_is_registered() {
        assert!(super::super::default_registry().get("checkpoint").is_some());
    }

    #[test]
    fn checkpoint_default_action_is_snapshot() {
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), std::env::temp_dir()));
        // Headless backend → BACKEND_REQUIRED envelope surfaces as Err; the point is
        // that `snapshot` is accepted (no "unknown action" Resolve error).
        let err = CheckpointBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::Backend(_)),
            "empty args must default to snapshot and reach the backend, got: {err:?}"
        );
    }

    #[test]
    fn checkpoint_unknown_action_is_a_clear_error() {
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), std::env::temp_dir()));
        let err = CheckpointBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @checkpoint action"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

`src/bridges/compress.rs` — `#[cfg(test)] mod tests` (verbatim):

    #[test]
    fn compress_is_registered() {
        assert!(super::super::default_registry().get("compress").is_some());
    }

    #[test]
    fn compress_default_action_reaches_backend() {
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), std::env::temp_dir()));
        let err = CompressBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::Backend(_)), "got: {err:?}");
    }

Gloss-Regressionstest — in `src/bridges/mod.rs` `#[cfg(test)] mod tests` (verbatim):

    #[test]
    fn checkpoint_and_compress_bridges_registered() {
        let reg = default_registry();
        assert!(reg.get("checkpoint").is_some(), "checkpoint bridge missing");
        assert!(reg.get("compress").is_some(), "compress bridge missing");
    }

Run: `cargo nextest run checkpoint compress` — Expected: FAIL (RED, Module/Structs existieren
noch nicht — kompiliert nicht; erst nach Schritt 2 grün).

### 2. GREEN — Implementierung

`src/bridges/checkpoint.rs` (verbatim, neuer Code — Kopf-Kommentar + Struct + impl; der
Test-Block aus Schritt 1 kommt darunter mit `use super::*; use crate::header::LeanMdHeader;`):

    //! `@checkpoint` bridge -> shadow-git safety net via `ctx_checkpoint` (design section
    //! "Neue Directive-Bridges"). Separate from the user's `.git`. Headless: outbound over the
    //! CodeIntelBackend, BACKEND_REQUIRED envelope discarded by callers. `action`
    //! (snapshot|log|diff|restore) is positional-0, default `snapshot`. Optional
    //! `label=`/`message=`. Byte-stable (#498).

    use std::rc::Rc;

    use super::{BridgeError, DirectiveBridge};
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;

    pub struct CheckpointBridge;

    impl DirectiveBridge for CheckpointBridge {
        fn name(&self) -> &'static str {
            "checkpoint"
        }

        fn execute(
            &self,
            ctx: &Rc<EngineContext>,
            args: &DirectiveArgs,
        ) -> Result<String, BridgeError> {
            let action = match args.positional(0).or_else(|| args.get("action")).unwrap_or("snapshot") {
                a @ ("snapshot" | "log" | "diff" | "restore") => a,
                other => {
                    return Err(BridgeError::Resolve(format!(
                        "unknown @checkpoint action '{other}'. Use: snapshot|log|diff|restore"
                    )));
                }
            };
            let mut payload = serde_json::Map::new();
            payload.insert("action".into(), action.into());
            if let Some(l) = args.get("label") {
                payload.insert("label".into(), l.into());
            }
            if let Some(m) = args.get("message") {
                payload.insert("message".into(), m.into());
            }
            let out = ctx
                .backend
                .call("ctx_checkpoint", serde_json::Value::Object(payload))
                .map_err(BridgeError::Backend)?;
            Ok(out)
        }
    }

`src/bridges/compress.rs` (verbatim, neuer Code; Test-Block darunter mit denselben `use`s):

    //! `@compress` bridge -> session-context checkpoint via `ctx_compress` (design section
    //! "Neue Directive-Bridges", #541). For long conversations. `action` positional-0, default
    //! `checkpoint`. Headless: outbound, BACKEND_REQUIRED envelope discarded. Byte-stable (#498).

    use std::rc::Rc;

    use super::{BridgeError, DirectiveBridge};
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;

    pub struct CompressBridge;

    impl DirectiveBridge for CompressBridge {
        fn name(&self) -> &'static str {
            "compress"
        }

        fn execute(
            &self,
            ctx: &Rc<EngineContext>,
            args: &DirectiveArgs,
        ) -> Result<String, BridgeError> {
            let action = args.positional(0).or_else(|| args.get("action")).unwrap_or("checkpoint");
            let payload = serde_json::json!({ "action": action });
            let out = ctx
                .backend
                .call("ctx_compress", payload)
                .map_err(BridgeError::Backend)?;
            Ok(out)
        }
    }

`src/bridges/mod.rs` — Modul-Deklarationen + Registrierung (verbatim ergänzen):

    // bei den `pub mod`-Zeilen (alphabetisch einsortiert):
    pub mod checkpoint;
    pub mod compress;

    // in default_registry():
    reg.register(Box::new(checkpoint::CheckpointBridge));
    reg.register(Box::new(compress::CompressBridge));

`content/gloss/directives.lmd.md` — zwei Rows in die Tabelle (verbatim, 2-Spalten-Format;
`{raw}` = alle Args). Diese zwei Zeilen sind NICHT direktiven-aktiv (Tabellen-Zellen), daher
inline hier zulässig:

| checkpoint       | Checkpoint (shadow-git) `{raw}`        |
| compress         | Compress session `{raw}`               |

Run: `cargo nextest run` — Expected: alle Tests PASS (GREEN), inkl. der neuen Bridge-Tests und
`default_registry_has_all_core_bridges` (Regression).

### 3. Verify & Close

@call verify(src/bridges/checkpoint.rs)
@call gate(src/bridges/checkpoint.rs src/bridges/compress.rs src/bridges/mod.rs content/gloss/directives.lmd.md)
@call commit("src/bridges/checkpoint.rs src/bridges/compress.rs src/bridges/mod.rs content/gloss/directives.lmd.md", "feat: checkpoint and compress directive bridges")
@call remember_decision("@checkpoint = shadow-git snapshot (ctx_checkpoint); @compress = session compaction (ctx_compress) — disambiguated names")

@phase-end

@phase "task-4"

## Task 4 — `on-complete`-Sink-Erweiterung: rename + `return`/`handoff`/`sync`

**Datei:** `src/phases.rs` (`parse_on_complete`, `fire_action`, `fire_agent`).
**Landing-unabhängig** (Sinks feuern direkt über den Backend, nicht über die T3-Bridges).

**Interfaces:**
- `on-complete=compress` (kanonisch) → `ctx_compress action=checkpoint`; deprecated Alias
  `on-complete=checkpoint` feuert weiter dasselbe (Back-Compat).
- `on-complete=return="<report>"` → `ctx_agent action=return message="<report>"`.
- `on-complete=handoff="<baton>" to_agent="<id>"` → `ctx_agent action=handoff message="<baton>"
  to_agent="<id>"`.
- `on-complete=sync` → `ctx_agent action=sync`.

**Anker (bestehend):** `@symbol parse_on_complete` (`src/phases.rs:77`), `@symbol fire_action`
(`src/phases.rs:111`), `@symbol fire_agent` (`src/phases.rs:178`). Beachte: `parse_on_complete`
hat heute einen Bare-Token-Sonderfall für `checkpoint` (L86-92), weil valuelose Tokens keine
`named_pairs` bilden — `compress`/`sync` brauchen dieselbe Behandlung.

### 1. RED — failing tests

In `src/phases.rs` `#[cfg(test)] mod tests` (verbatim; nutzt den bestehenden
`recording_ctx`/`find_call`-Harness, Anker `@symbol recording_ctx`, `@symbol find_call`):

    #[test]
    fn oncomplete_compress_sink_fires_ctx_compress() {
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@phase \"P\"\nbody\n@on complete=compress\n@phase-end\n";
        let _ = render_with_phases(&ctx, doc);
        let c = calls.borrow();
        assert!(
            find_call(&c, "ctx_compress", "checkpoint").is_some(),
            "compress sink must fire ctx_compress action=checkpoint: {c:?}"
        );
    }

    #[test]
    fn oncomplete_checkpoint_alias_still_fires() {
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@phase \"P\"\nbody\n@on complete=checkpoint\n@phase-end\n";
        let _ = render_with_phases(&ctx, doc);
        let c = calls.borrow();
        assert!(
            find_call(&c, "ctx_compress", "checkpoint").is_some(),
            "deprecated checkpoint alias must still fire ctx_compress: {c:?}"
        );
    }

    #[test]
    fn oncomplete_return_sink_fires_ctx_agent_return() {
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@phase \"P\"\nbody\n@on complete=return=\"status: DONE\"\n@phase-end\n";
        let _ = render_with_phases(&ctx, doc);
        let c = calls.borrow();
        let call = find_call(&c, "ctx_agent", "return").expect("return sink must fire ctx_agent");
        assert_eq!(call["message"], "status: DONE");
    }

    #[test]
    fn oncomplete_handoff_sink_passes_to_agent() {
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@phase \"P\"\nbody\n@on complete=handoff=\"baton text\" to_agent=\"ctrl-1\"\n@phase-end\n";
        let _ = render_with_phases(&ctx, doc);
        let c = calls.borrow();
        let call = find_call(&c, "ctx_agent", "handoff").expect("handoff sink must fire ctx_agent");
        assert_eq!(call["message"], "baton text");
        assert_eq!(call["to_agent"], "ctrl-1");
    }

    #[test]
    fn oncomplete_sync_sink_fires() {
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@phase \"P\"\nbody\n@on complete=sync\n@phase-end\n";
        let _ = render_with_phases(&ctx, doc);
        let c = calls.borrow();
        assert!(find_call(&c, "ctx_agent", "sync").is_some(), "sync sink must fire: {c:?}");
    }

    #[test]
    fn post_diary_sinks_unchanged() {
        // Regression: the existing post/diary sinks keep firing ctx_agent.
        let (ctx, calls) = recording_ctx(std::env::temp_dir());
        let doc = "@phase \"P\"\nbody\n@on complete=post=\"hi\" category=status\n@phase-end\n";
        let _ = render_with_phases(&ctx, doc);
        assert!(find_call(&calls.borrow(), "ctx_agent", "post").is_some());
    }

Run: `cargo nextest run oncomplete` — Expected: `compress`/`return`/`handoff`/`sync` FAIL (RED);
`oncomplete_checkpoint_alias_still_fires` + `post_diary_sinks_unchanged` grün (bestehendes
Verhalten).

### 2. GREEN — Implementierung

`parse_on_complete` — den Bare-Token-Sonderfall verallgemeinern (verbatim ersetzt L86-92):

    // Bare (valueless) sinks that carry no `key=value` pair.
    if matches!(head, "checkpoint" | "compress" | "sync") {
        return Some(OnComplete {
            sink: head.to_string(),
            value: String::new(),
            attrs: vec![],
        });
    }

`fire_agent` — Signatur um `to_agent` erweitern (verbatim ersetzt `@symbol fire_agent`):

    fn fire_agent(
        ctx: &Rc<EngineContext>,
        action: &str,
        message: &str,
        category: Option<&str>,
        to_agent: Option<&str>,
    ) {
        let mut payload = serde_json::Map::new();
        payload.insert("action".into(), action.into());
        payload.insert("message".into(), message.into());
        if let Some(category) = category {
            payload.insert("category".into(), category.into());
        }
        if let Some(to_agent) = to_agent {
            payload.insert("to_agent".into(), to_agent.into());
        }
        let _ = ctx
            .backend
            .call("ctx_agent", serde_json::Value::Object(payload));
    }

`fire_action` — die `post`/`diary`-Arme um den vierten Parameter ergänzen und die neuen Arme
hinzufügen (verbatim; ersetzt die bestehenden `post`/`diary`-Arme und den `checkpoint`-Arm):

    "post" => fire_agent(ctx, "post", &value, attr(&action.attrs, "category"), None),
    "diary" => fire_agent(ctx, "diary", &value, attr(&action.attrs, "category"), None),
    "return" => fire_agent(ctx, "return", &value, attr(&action.attrs, "category"), None),
    "handoff" => fire_agent(ctx, "handoff", &value, None, attr(&action.attrs, "to_agent")),
    "sync" => fire_agent(ctx, "sync", "", None, None),
    "compress" | "checkpoint" => {
        // Canonical `compress`; deprecated alias `checkpoint`. Both compact the live
        // session via ctx_compress. Headless → discarded BACKEND_REQUIRED (#498).
        let _ = ctx
            .backend
            .call("ctx_compress", serde_json::json!({ "action": "checkpoint" }));
    }

Run: `cargo nextest run` — Expected: alle Tests PASS (GREEN), inkl.
`on_complete_fires_session_sinks_in_order_on_clean_end` (Regression).

### 3. Verify & Close

@call verify(src/phases.rs)
@call gate(src/phases.rs)
@call commit("src/phases.rs", "feat: on-complete return/handoff/sync sinks + compress rename (checkpoint alias)")
@call remember_decision("on-complete sinks: compress (alias checkpoint)->ctx_compress; return/handoff/sync->ctx_agent; fire_agent now carries to_agent")

@phase-end

@phase "task-5"

## Task 5 — Plan-Recipes `snapshot()` / `compress()` / `task_return()` (Verifikation)

**Datei:** `content/templates/plan-recipes.lmd.md` (Recipes **bereits angehängt** — s. Meta-head
Architecture). **Depends:** Task 3 (`@checkpoint`), Task 4 (`on-complete=return`), **+ Bug-1-Fix**
(extern) für `task_return`s quoted Ein-Arg mit Binnen-Kommas/Semikolons.

Dies ist ein **Verifikations-Gate**, kein Content-Schreib-Task: die drei `@define`s
(`snapshot`/`compress`/`task_return`, je mit HTML-Kommentar-Erstzeile) sind vor-materialisiert,
weil ihr Body zeilenanfängliche `@checkpoint`/`@compress`/`@on complete` und `{{ … }}`-Vars trägt,
die im gerenderten Brief nicht darstellbar sind. Der Task verifiziert die Expansion per Test.

**Anker (bestehend):** `lean-md source content/templates/plan-recipes.lmd.md` — die drei neuen
`@define`s stehen nach `render_check`; jede trägt die HTML-Kommentar-Erstzeile (Index-Gate
`plan_recipes_all_documented`).

### 1. Test schreiben

Erstelle `tests/sdd_recipes.rs` (verbatim, neuer Code) — rendert Mini-Pläne, die die drei Recipes
aus der **echten** Recipe-Datei importieren:

    //! SDD recipes expand to the right directives/sinks (design section "Neue Directive-Bridges").
    use std::process::Command;

    fn render_from_repo(plan_body: &str) -> String {
        // Run from a temp project root with the real recipe library copied in, so
        // `@import .lean-ctx/lean-md/plan-recipes /` resolves (same path real plans use).
        let dir = std::env::temp_dir().join(format!("lmd_sddrec_{}", std::process::id()));
        let recipes_dir = dir.join(".lean-ctx/lean-md");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/content/templates/plan-recipes.lmd.md"),
            recipes_dir.join("plan-recipes.lmd.md"),
        )
        .unwrap();
        std::fs::create_dir_all(recipes_dir.join("lang")).unwrap();
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/content/lang/rust.lmd.md"),
            recipes_dir.join("lang/rust.lmd.md"),
        )
        .unwrap();
        let plan = dir.join("p.lmd.md");
        std::fs::write(&plan, plan_body).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_lean-md"))
            .args(["render", plan.to_str().unwrap()])
            .current_dir(&dir)
            .output()
            .expect("run lean-md");
        let _ = std::fs::remove_dir_all(&dir);
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[test]
    fn recipe_snapshot_expands_to_checkpoint() {
        let out = render_from_repo(
            "@lean-md 0.4\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n\n@call snapshot(\"pre-task-3\") /\n",
        );
        assert!(out.contains("@checkpoint action=snapshot"), "got: {out}");
        assert!(out.contains("pre-task-3"), "label must survive: {out}");
    }

    #[test]
    fn recipe_compress_expands() {
        let out = render_from_repo(
            "@lean-md 0.4\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n\n@call compress() /\n",
        );
        assert!(out.contains("@compress action=checkpoint"), "got: {out}");
    }

    #[test]
    fn recipe_task_return_expands_with_commas() {
        // Requires the Bug-1 fix: the single quoted arg carries inner commas/semicolons.
        let out = render_from_repo(
            "@lean-md 0.4\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n\n@call task_return(\"status: DONE; commits: a1b2c3, d4e5f6\") /\n",
        );
        assert!(out.contains("on-complete=return"), "got: {out}");
        assert!(out.contains("status: DONE; commits: a1b2c3, d4e5f6"), "arg intact: {out}");
    }

### 2. Erwartung (Verifikations-Gate, kein RED-first)

Da die Recipes vor-materialisiert sind und die Bridges (T3) + Sink (T4) gelandet sind, sind
`recipe_snapshot_expands_to_checkpoint` und `recipe_compress_expands` **grün**. Der dritte Test
`recipe_task_return_expands_with_commas` ist genau dann grün, wenn der **externe Bug-1-Fix**
gelandet ist — ohne ihn zerreißt das Binnen-Komma das Arg (Prerequisite-Nachweis). Setzt Bug-1
also voraus.

Run: `cargo nextest run --test sdd_recipes` — Expected: alle 3 PASS (setzt Bug-1 + T3 + T4 voraus).
Run: `cargo nextest run` — Expected: volle Suite PASS, inkl. `plan_recipes_all_documented` /
`no_orphan_call` (die drei neuen `@define`s tragen je die HTML-Kommentar-Erstzeile).

### 3. Verify & Close

@call verify(tests/sdd_recipes.rs)
@call gate(content/templates/plan-recipes.lmd.md tests/sdd_recipes.rs)
@call commit("content/templates/plan-recipes.lmd.md tests/sdd_recipes.rs", "test: verify snapshot/compress/task_return recipe expansion")
@call remember_decision("SDD recipes snapshot/compress/task_return are pre-appended to plan-recipes.lmd.md; task_return needs the Bug-1 quote-aware arg split")

@phase-end

@phase "task-6"

## Task 6 — SDD-Seeds verdrahten (Body + SKILL.md + 3 Companions + Install)

**Content liegt bereits am Zielort** unter `content/skills/lmd-subagent-driven-development/`
(Body, SKILL.md, 3 Companions — s. Meta-head). Dieser Task **registriert** ihn und verifiziert
per Test. Companions und Body werden im **selben** Task registriert, weil der Body
`companion="…"` nennt und `no_dangling_companion_refs_in_seeds` verlangt, dass jede Referenz in
`COMPANIONS` aufgelöst ist.

**Dateien:** `src/skills.rs` (4 Consts + `SKILLS`-Row + 3 `COMPANIONS`-Rows), `src/skill_install.rs`
(`SKILL_MD`-Const + `INSTALLABLE_SKILLS`-Row), `src/availability.rs` (`COVERAGE`-Rows),
`src/bridges/dispatch.rs` (crp-compact-Test). **Depends:** Task 2 (`--list-phases`), Task 3/4/5
(der Body nutzt `@checkpoint`/`@compress` bzw. `@call snapshot/compress/task_return`).

**Anker (bestehend):** `SKILLS`/`COMPANIONS` (`src/skills.rs:55,80`), `@symbol skill_body`,
`@symbol companion_body`, `INSTALLABLE_SKILLS` (`src/skill_install.rs:15`), `@symbol skill_md`,
`COVERAGE` (`src/availability.rs`), `@symbol no_dangling_companion_refs_in_seeds`
(`src/skills.rs:763`), `@symbol phases_carry_next_pointers` (`src/skills.rs:1042`),
`dispatch_threads_crp_tdd_into_contract` (`src/bridges/dispatch.rs`). Der Seed-Content wird bei
Bedarf raw gelesen: `lean-md source content/skills/lmd-subagent-driven-development/body.lmd.md`.

### 1. RED — failing tests

In `src/skills.rs` `#[cfg(test)] mod tests` (verbatim):

    #[test]
    fn sdd_all_phases_render_nonempty() {
        for phase in ["orient", "preflight", "dispatch", "review", "final-review", "handoff"] {
            let out = render_skill(
                "lmd-subagent-driven-development",
                Some(phase),
                Some(Consumer::Ai),
                None,
                std::env::temp_dir(),
            )
            .unwrap_or_else(|e| panic!("phase {phase} failed: {e}"));
            assert!(!out.trim().is_empty(), "phase {phase} rendered empty");
        }
    }

    #[test]
    fn sdd_phase_isolation_no_cross_phase_leak() {
        let orient = render_skill(
            "lmd-subagent-driven-development",
            Some("orient"),
            Some(Consumer::Ai),
            None,
            std::env::temp_dir(),
        )
        .unwrap();
        // The final-review-only marker must not leak into orient.
        assert!(!orient.contains("code-reviewer"), "cross-phase leak: {orient}");
    }

    #[test]
    fn sdd_render_is_byte_stable() {
        let a = render_skill("lmd-subagent-driven-development", Some("dispatch"), Some(Consumer::Ai), None, std::env::temp_dir()).unwrap();
        let b = render_skill("lmd-subagent-driven-development", Some("dispatch"), Some(Consumer::Ai), None, std::env::temp_dir()).unwrap();
        assert_eq!(a, b, "SDD render must be byte-stable (#498)");
    }

    #[test]
    fn sdd_companions_resolve() {
        for c in ["implementer", "task-reviewer", "code-reviewer"] {
            let out = render_companion(
                "lmd-subagent-driven-development",
                c,
                Some(Consumer::Ai),
                None,
                std::env::temp_dir(),
            )
            .unwrap_or_else(|e| panic!("companion {c} failed: {e}"));
            assert!(!out.trim().is_empty(), "companion {c} rendered empty");
        }
    }

    #[test]
    fn sdd_dispatch_implementer_composes() {
        // @dispatch to the implementer prepends the dispatch contract + bootstrap.
        let doc = "@dispatch skill=\"lmd-subagent-driven-development\" companion=\"implementer\" role=dev to_agent=\"c\"\n";
        let out = crate::engine::render(doc);
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(out.contains("ToolSearch(query=\"select:mcp__lean-ctx__ctx_read"), "bootstrap missing: {out}");
    }

In `src/skill_install.rs` `#[cfg(test)] mod tests` (verbatim, Muster wie
`writing_plans_install_writes_skill_md`):

    #[test]
    fn sdd_install_writes_skill_md() {
        let root = std::env::temp_dir().join(format!("lmd_sdd_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md =
            install_skill("lmd-subagent-driven-development", Scope::Local, &root, false).unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-subagent-driven-development"),
            "stub frontmatter missing"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

In `src/bridges/dispatch.rs` `#[cfg(test)] mod tests` (verbatim, neben dem bestehenden `_tdd_`-Test):

    #[test]
    fn dispatch_threads_crp_compact_into_contract() {
        let doc = "@lean-md\ncrp: compact\n\n@phase \"P\"\nDo the work.\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("CRP mode `compact`"), "crp compact threaded: {out}");
        assert!(!out.contains("{{ crp }}"), "placeholder substituted: {out}");
    }

Erweitere den bestehenden `phases_carry_next_pointers`-Test (Anker) um die SDD-Kette: jede
Nicht-Terminal-Phase (`orient`→…→`final-review`) trägt einen `next:`-Pointer; `handoff` ist terminal.

Run: `cargo nextest run sdd_ dispatch_threads_crp_compact` — Expected: FAIL (RED, Skill/Companions
nicht registriert).

### 2. GREEN — Registrierung

`src/skills.rs` — vier `include_str!`-Consts + `SKILLS`-Row + drei `COMPANIONS`-Rows (verbatim
ergänzen; alle vier Zieldateien existieren bereits am Zielort):

    const LMD_SDD_BODY: &str =
        include_str!("../content/skills/lmd-subagent-driven-development/body.lmd.md");
    const LMD_SDD_IMPLEMENTER: &str =
        include_str!("../content/skills/lmd-subagent-driven-development/companions/implementer.lmd.md");
    const LMD_SDD_TASK_REVIEWER: &str =
        include_str!("../content/skills/lmd-subagent-driven-development/companions/task-reviewer.lmd.md");
    const LMD_SDD_CODE_REVIEWER: &str =
        include_str!("../content/skills/lmd-subagent-driven-development/companions/code-reviewer.lmd.md");

    // in SKILLS:
    ("lmd-subagent-driven-development", LMD_SDD_BODY),

    // in COMPANIONS:
    ("lmd-subagent-driven-development", "implementer", LMD_SDD_IMPLEMENTER),
    ("lmd-subagent-driven-development", "task-reviewer", LMD_SDD_TASK_REVIEWER),
    ("lmd-subagent-driven-development", "code-reviewer", LMD_SDD_CODE_REVIEWER),

`src/skill_install.rs` — `SKILL_MD`-Const + `INSTALLABLE_SKILLS`-Row (verbatim ergänzen; SDD hat
keine `ASSETS`):

    const SDD_SKILL_MD: &str =
        include_str!("../content/skills/lmd-subagent-driven-development/SKILL.md");

    // in INSTALLABLE_SKILLS:
    ("lmd-subagent-driven-development", SDD_SKILL_MD),

`src/availability.rs` — `COVERAGE`-Rows (verbatim ergänzen; erstes Feld = voller Skill-Name;
Spalte 3 = registriertes Directive, seit T3/T4 vorhanden):

    ("lmd-subagent-driven-development", "dispatch", "dispatch", "fragment-compose"),
    ("lmd-subagent-driven-development", "dispatch", "checkpoint", "ctx_checkpoint"),
    ("lmd-subagent-driven-development", "review", "dispatch", "fragment-compose"),
    ("lmd-subagent-driven-development", "final-review", "dispatch", "fragment-compose"),
    ("lmd-subagent-driven-development", "final-review", "review", "ctx_review"),
    ("lmd-subagent-driven-development", "final-review", "smells", "ctx_smells"),
    ("lmd-subagent-driven-development", "handoff", "compress", "ctx_compress"),
    ("lmd-subagent-driven-development", "implementer", "dispatch", "fragment-compose"),
    ("lmd-subagent-driven-development", "task-reviewer", "dispatch", "fragment-compose"),
    ("lmd-subagent-driven-development", "code-reviewer", "dispatch", "fragment-compose"),

Run: `cargo nextest run` — Expected: alle Tests PASS (GREEN), inkl. `sdd_*`,
`dispatch_threads_crp_compact_into_contract`, `sdd_install_writes_skill_md`,
`no_dangling_companion_refs_in_seeds` (Body+Companions jetzt gemeinsam registriert),
`every_covered_directive_is_registered`, und der Fragment-Konsistenz-Gate (built-in == on-disk
für Body und alle drei Companions).

### 3. Verify, Fidelity-Matrix & Close

**Fidelity-Matrix (No-Function-Loss-Nachweis, Plan-Task-Ebene — kein cargo-Gate):** kein
superpowers-SDD-Abschnitt ohne Landepunkt:

- When-to-Use / Tool-Discipline / Isolation / Resume → `orient`.
- Pre-Flight-Konflikt-Scan + Task-Enumeration (`--list-phases`) → `preflight`.
- Process / Model-Selection / Status-Handling / Continuous-Execution → `dispatch`.
- ⚠️-Items / Reviewer-Prompt-Discipline / Fix-Loop / Durable-Progress → `review` + `task-reviewer`.
- Final-Review (Pre-Pass + Urteil) → `final-review` + `code-reviewer`.
- File-Handoffs→lean-ctx → Datenfluss (`ctx_agent`/`ctx_session`/`ctx_knowledge`) über alle Phasen.
- Implementer-Prompt (TDD / Self-Review / Escalation) → `implementer` companion.

@call render_check("lmd-subagent-driven-development", "dispatch")
@call verify(src/skills.rs)
@call gate(src/skills.rs src/skill_install.rs src/availability.rs src/bridges/dispatch.rs)
@call commit("src/skills.rs src/skill_install.rs src/availability.rs src/bridges/dispatch.rs", "feat: register lmd-subagent-driven-development skill (body, companions, install, coverage, crp test)")
@call remember_decision("SDD skill fully wired: skills.rs (body+3 companions), skill_install.rs (INSTALLABLE_SKILLS), availability.rs (COVERAGE); companions + body registered together to keep no_dangling_companion_refs_in_seeds green")

@phase-end
