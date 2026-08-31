@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# `@read` ohne Sitzung — Pakete B + C (lean-md)

Spec: `docs/lean-md/specs/2026-08-31-lmd-anker-ohne-sitzung-design.md` §2.2 (B) und §2.3 (C).

## Goal

`@read`-Anker sollen einen sitzungslosen `ctx_read` überleben, statt die ganze Phase
mitsamt ihren `@on complete`-Sinks zu töten (B); und der Gateway-Renderpfad soll
Phasen isolieren, projektrelativ jailen und sich nicht selbst per Sink-Rekursion
abschießen (C).

## Architecture

- Fehlerklassen: `BridgeError::Backend` (`src/bridges/mod.rs:60`) ist heute immer
  fatal in einer Phase (`src/phases.rs:372-395`). Neu trennt
  `DirectiveBridge::read_only()` lesende von schreibenden Bridges;
  `render::dispatch_result` (`src/render.rs:24-34`) degradiert nur die lesenden.
- Ersatzdaten kommen weiter ausschließlich über `ctx.backend` (`src/backend.rs:29-32`);
  lean-md bekommt keinen lokalen Lesepfad.
- Gateway-Pfad = `cmd_mcp` (`src/bin/lean_md.rs:670`) → `mcp_load_source` (`:557`)
  → `do_render` (`:39`) → `render_source_with_phase` (`src/skills.rs:158`).

## Global Constraints

- Non-Goal: Paket A (lean-ctx `oneshot_ctx`) ist **nicht** Teil dieses Plans; ebenso
  wenig das `mcp`-Feature / `McpBackend` (im Release nicht kompiliert).
- Non-Goal (Spec §2.2 B3): keine neue Direktiven-Syntax — `@read … symbol=X` wird
  nicht eingeführt, `@symbol body name=X` deckt den Fall ab.
- Kein lokaler Lesepfad in lean-md — jede Ersatzausgabe stammt aus einem
  `ctx.backend.call(...)` (server-seitig gejailt/redigiert).
- Byte-Stabilität (#498) ist Testgate: Fallback-Texte sind reine Funktionen ihrer
  Argumente, keine Zeitstempel/Zähler. Task 2 prüft das explizit.
- Das Crate ist deutschfrei (`docs/lean-md/specs/2026-06-30-crate-deutschfrei-design.md`):
  die deutschen Marker-Strings der Design-Spec werden sinngleich **englisch**
  umgesetzt. Substanz unverändert, Wortlaut abweichend.
- Nur `BridgeError::Backend` degradiert. Autorenfehler (`MissingArg`, `Resolve`/Jail,
  `ShellDenied`) brechen die Phase unverändert ab.
- Reihenfolge: Task 1 vor Task 2 (Task 2 setzt den Degradationskontrakt voraus),
  Task 3 vor Task 4 (Task 4 testet gegen die neue fünfstellige `do_render`-Signatur).
  Task 5 ist von allen unabhängig.
- Rückwärtskompatibilität der Fehlersprache: bestehende e2e-Tests prüfen auf den
  Wortlaut `BACKEND_REQUIRED` (u. a. `src/engine.rs:574-591`). Die Degradationsnotiz
  aus Task 1 trägt deshalb den `BridgeError`-Display, nicht den nackten `BackendError`.
- Dieser Plan benutzt bewusst **keine** aktiven `@read`/`@symbol`-Direktiven als Anker,
  sondern `path:line`-Textanker: solange Paket B nicht steht, würde ein Anker beim
  Rendern der Task-Phase genau den Bug auslösen, den der Plan behebt.

@phase "task-1"
## Task 1 — B1: lesende Bridges degradieren, statt die Phase abzubrechen

**Files:** `src/bridges/mod.rs` (Trait + Registry-Test), `src/render.rs`
(`dispatch_result`, neue `degraded_note`), `src/phases.rs` (Test), plus je eine
Methode in den 15 lesenden Bridge-Dateien.

**Interfaces:** Produces `DirectiveBridge::read_only(&self) -> bool` (Default `false`).
Consumes `BridgeError::Backend` / `BackendError` (`src/backend.rs:9-25`).

**Anchors:** `src/bridges/mod.rs:89-98` (Trait; `accepts_pipe` ist das Muster) ·
`src/render.rs:24-34` (`dispatch_result`) · `src/phases.rs:372-395` (die Abbruchstelle,
die für lesende Bridges nicht mehr feuern darf) · `src/bridges/mod.rs:120-153`
(Registry — Quelle der Namensliste).

### Test zuerst

In `src/phases.rs`, Test-Modul am Dateiende — gemeinsamer Recorder + zwei Tests:

    struct Recorder {
        calls: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
        fail: &'static str,
    }
    impl crate::backend::CodeIntelBackend for Recorder {
        fn call(
            &self,
            tool: &str,
            _args: serde_json::Value,
        ) -> Result<String, crate::backend::BackendError> {
            self.calls.borrow_mut().push(tool.to_string());
            if tool == self.fail {
                return Err(crate::backend::BackendError::NonZero {
                    code: 2,
                    stderr: "error: -32603: session not available".into(),
                });
            }
            Ok(String::new())
        }
    }

    #[test]
    fn a_read_only_backend_failure_does_not_abort_the_phase() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = Rc::new(crate::engine::EngineContext::with_backend(
            crate::header::LeanMdHeader::default(),
            std::path::PathBuf::from("."),
            Box::new(Recorder {
                calls: calls.clone(),
                fail: "ctx_read",
            }),
        ));
        let src = "@phase \"t1\"\n@read src/lib.rs mode=full\nAFTER-TEXT\n@on complete decision=\"t1 done\"\n@phase-end\n";
        let out = render_with_phases(&ctx, src);
        assert!(!out.contains("PHASE_ABORTED"), "{out}");
        assert!(out.contains("AFTER-TEXT"), "body after the anchor must survive: {out}");
        assert!(
            calls.borrow().iter().any(|t| t == "ctx_session"),
            "@on complete must still fire: {:?}",
            calls.borrow()
        );
    }

    #[test]
    fn a_writing_bridge_still_aborts_the_phase() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = Rc::new(crate::engine::EngineContext::with_backend(
            crate::header::LeanMdHeader::default(),
            std::path::PathBuf::from("."),
            Box::new(Recorder {
                calls: calls.clone(),
                fail: "ctx_edit",
            }),
        ));
        let src = "@phase \"t1\"\n@edit src/lib.rs old=\"a\" new=\"b\"\nAFTER-TEXT\n@phase-end\n";
        let out = render_with_phases(&ctx, src);
        assert!(out.contains("PHASE_ABORTED"), "a write must stay fatal: {out}");
    }

In `src/bridges/mod.rs`, Test-Modul — die Kontraktmatrix:

    #[test]
    fn read_only_matrix_matches_the_contract() {
        let reg = default_registry();
        for n in [
            "read", "search", "symbol", "find", "graph", "impact", "outline", "repomap",
            "architecture", "smells", "review", "routes", "inspect", "recall", "list",
        ] {
            assert!(reg.get(n).expect(n).read_only(), "{n} must be read-only");
        }
        for n in [
            "edit", "refactor", "reformat", "dispatch", "handoff", "remember", "checkpoint",
            "query",
        ] {
            assert!(!reg.get(n).expect(n).read_only(), "{n} must NOT be read-only");
        }
    }

@call tdd(a_read_only_backend_failure_does_not_abort_the_phase)

### Implementierung

`src/bridges/mod.rs` — Trait-Methode direkt hinter `accepts_pipe` (`:95-97`):

    /// Whether this bridge only READS (no write, no state change). A
    /// `BridgeError::Backend` from a read-only bridge degrades to a visible note in
    /// `render::dispatch_result` instead of aborting the enclosing `@phase`: an
    /// infrastructure outage must not swallow the rest of the phase and its
    /// `@on complete` sinks. Default `false` — writing bridges keep aborting.
    fn read_only(&self) -> bool {
        false
    }

`src/render.rs` — `dispatch_result` ersetzen und die Notiz danebenstellen:

    pub(crate) fn dispatch_result(
        ctx: &Rc<EngineContext>,
        name: &str,
        raw_args: &str,
    ) -> std::result::Result<String, super::bridges::BridgeError> {
        let args = DirectiveArgs::parse(raw_args);
        match ctx.registry.get(name) {
            Some(bridge) => match bridge.execute(ctx, &args) {
                // Read-only + backend outage → degrade (I2 abort stays for writes).
                Err(e @ super::bridges::BridgeError::Backend(_)) if bridge.read_only() => {
                    Ok(degraded_note(name, &e))
                }
                other => other,
            },
            None => Ok(resolve_value(ctx, name, raw_args)),
        }
    }

    /// Visible stand-in for a read-only directive whose backend call failed.
    /// Carries the `BridgeError` display, so the note keeps the historic
    /// `BACKEND_REQUIRED:` wording the e2e tests assert on.
    /// Byte-stable (#498): a pure function of (name, error) — no timestamp/counter.
    fn degraded_note(name: &str, e: &super::bridges::BridgeError) -> String {
        format!(
            "<!-- lmd:@{} unavailable: {} -->",
            sanitize_comment(name),
            sanitize_comment(&format!("{e}"))
        )
    }

In jeder der 15 Dateien `src/bridges/{read,search,symbol,find,graph,impact,outline,repomap,architecture,smells,review,routes,inspect,recall,list}.rs`
im `impl DirectiveBridge for …`-Block, neben `fn name`:

    fn read_only(&self) -> bool {
        true
    }

@call test(read_only_matrix_matches_the_contract)
@call test(a_writing_bridge_still_aborts_the_phase)

### Verify & Close

@call verify(src/render.rs src/bridges/mod.rs src/phases.rs)
@call review_change()
@call gate(src/)
@call commit("src/", "feat(bridges): read-only bridges degrade instead of aborting a phase")
@call remember_decision("BridgeError::Backend from a read_only() bridge degrades to a visible note in dispatch_result; writes stay fatal")
@phase-end

@phase "task-2"
## Task 2 — B2: `@read`-Fallback über sitzungsfreie Tools

@call recall_context("read_only degradation contract")

**Files:** `src/bridges/read.rs`.

**Interfaces:** `ReadBridge::execute` gibt bei Backendfehler `Ok(<Ersatz>)` statt
`Err`. Neue freie Funktion `read_fallback(ctx, path, mode) -> String`.
`@symbol body` bleibt unangetastet (läuft über `ctx_search`, keine Sitzung nötig).

**Anchors:** `src/bridges/read.rs:17-35` (`execute`, heute `.map_err(BridgeError::Backend)?`) ·
`src/bridges/outline.rs:34-45` (die `ctx_outline`-Payload-Form, die der Fallback spiegelt).

**Mode-Matrix (Spec §2.2 B2):** `signatures`/`map`/`auto` → ein `ctx_outline`-Call,
alles andere (`full`, `lines:N-M`, `anchored`, `raw`, …) → sichtbare Leseanweisung,
kein zweiter Backendcall.

### Test zuerst

`src/bridges/read.rs`, Test-Modul — Recorder + drei Tests:

    struct FailingRead {
        calls: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    }
    impl crate::backend::CodeIntelBackend for FailingRead {
        fn call(
            &self,
            tool: &str,
            _args: serde_json::Value,
        ) -> Result<String, crate::backend::BackendError> {
            self.calls.borrow_mut().push(tool.to_string());
            match tool {
                "ctx_outline" => Ok("OUTLINE_OK\n".to_string()),
                _ => Err(crate::backend::BackendError::NonZero {
                    code: 2,
                    stderr: "error: -32603: session not available".into(),
                }),
            }
        }
    }

    fn failing_ctx(calls: &std::rc::Rc<std::cell::RefCell<Vec<String>>>) -> Rc<EngineContext> {
        Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            PathBuf::from("."),
            Box::new(FailingRead {
                calls: calls.clone(),
            }),
        ))
    }

    #[test]
    fn signatures_mode_falls_back_to_outline_exactly_once() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/lib.rs mode=signatures"))
            .unwrap();
        assert!(out.contains("fallback=ctx_outline"), "{out}");
        assert!(out.contains("OUTLINE_OK"), "{out}");
        assert_eq!(
            calls.borrow().iter().filter(|t| *t == "ctx_outline").count(),
            1,
            "exactly one substitute call: {:?}",
            calls.borrow()
        );
    }

    #[test]
    fn full_mode_renders_a_self_read_order_without_a_second_call() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/seal.rs mode=full"))
            .unwrap();
        assert!(out.contains("src/seal.rs"), "{out}");
        assert!(out.contains("mode=\"full\""), "the order must be copy-pasteable: {out}");
        assert!(
            !calls.borrow().iter().any(|t| t == "ctx_outline"),
            "no substitute exists for full: {:?}",
            calls.borrow()
        );
    }

    #[test]
    fn the_read_fallback_is_byte_stable() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let args = DirectiveArgs::parse("src/lib.rs mode=signatures");
        let a = ReadBridge.execute(&ctx, &args).unwrap();
        let b = ReadBridge.execute(&ctx, &args).unwrap();
        assert_eq!(a, b, "#498: two renders of the same source are identical");
    }

@call tdd(signatures_mode_falls_back_to_outline_exactly_once)

### Implementierung

`src/bridges/read.rs` — den `?`-Pfad in `execute` ersetzen und `read_fallback`
hinter den `impl`-Block setzen:

            let mode = args.get("mode").unwrap_or("auto");
            match ctx.backend.call(
                "ctx_read",
                serde_json::json!({ "path": path, "mode": mode }),
            ) {
                Ok(out) => Ok(out),
                // A session-less `lean-ctx call ctx_read` fails here (design 2026-08-31
                // §1.1). Degrade instead of losing the anchor — the enclosing @phase and
                // its @on complete sinks must survive an infrastructure outage.
                Err(_) => Ok(read_fallback(ctx, path, mode)),
            }
        }
    }

    /// Session-free substitute for a failed `ctx_read` (design 2026-08-31 §2.2 B2).
    /// `ctx_outline` needs no session and answers the orientation modes with
    /// line-numbered signatures; every other mode has no session-free equivalent, so
    /// the anchor renders as a visible self-read order rather than a silent comment.
    /// Byte-stable (#498): a pure function of (path, mode).
    fn read_fallback(ctx: &Rc<EngineContext>, path: &str, mode: &str) -> String {
        if matches!(mode, "signatures" | "map" | "auto")
            && let Ok(out) = ctx
                .backend
                .call("ctx_outline", serde_json::json!({ "path": path }))
        {
            return format!(
                "<!-- lmd:@read fallback=ctx_outline (ctx_read needs a session) -->\n{out}"
            );
        }
        format!(
            "> ⚠ @read {path} mode={mode} — backend has no session.\n\
             >   Read it yourself: ctx_read(path=\"{path}\", mode=\"{mode}\")\n"
        )
    }

@call test(full_mode_renders_a_self_read_order_without_a_second_call)
@call test(the_read_fallback_is_byte_stable)

### Verify & Close

@call verify(src/bridges/read.rs)
@call gate(src/bridges/read.rs)
@call commit("src/bridges/read.rs", "feat(read): fall back to ctx_outline / a visible self-read order when ctx_read has no session")
@call remember_decision("@read degrades: signatures|map|auto -> one ctx_outline call with a fallback marker; every other mode -> a visible self-read order")
@phase-end

@phase "task-3"
## Task 3 — C1: `phase` im `path`/`content`-Zweig des MCP-Renders

**Files:** `src/bin/lean_md.rs`.

**Interfaces:** `do_render` bekommt `phase: Option<&str>` und liefert
`Result<String, lean_md::skills::SkillRenderError>`.

**Anchors:** `src/bin/lean_md.rs:39-48` (`do_render`, verdrahtet heute `None`) ·
`:795-820` (der `else`-Zweig, der `phase` ignoriert) · `:762-794` (der `skill`-Zweig
als Vorbild für die `-32602`-Meldung) · `src/skills.rs:83-104` (`SkillRenderError`).

**Verhalten:** nur `PhaseNotFound` wird `-32602`. `DuplicatePhase` bleibt eine
Render-Notiz im *Ergebnis* — dieselbe Linie wie bei `ctx_md_check` (`:826-828`),
sonst ändert sich die Drahtausgabe für bestehende Aufrufer.

**Grenze:** Der Handler liegt inline in der `cmd_mcp`-Leseschleife und ist nicht
direkt testbar; getestet wird `do_render` (das den Phasenschnitt trägt), das
Fehler-Mapping ist im Zweig zwei Zeilen darüber sichtbar.

### Test zuerst

`src/bin/lean_md.rs`, Test-Modul:

    #[test]
    fn mcp_whole_doc_render_honours_the_phase_argument() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t1\"\nONE\n@phase-end\n@phase \"t2\"\nTWO\n@phase-end\n";
        let out = do_render(src, std::path::PathBuf::from("."), None, None, Some("t1")).unwrap();
        assert!(out.contains("ONE"), "{out}");
        assert!(!out.contains("TWO"), "phase isolation must not leak the sibling: {out}");

        let err = do_render(src, std::path::PathBuf::from("."), None, None, Some("nope")).unwrap_err();
        assert!(
            matches!(err, lean_md::skills::SkillRenderError::PhaseNotFound(_)),
            "unknown phase must be a caller error: {err:?}"
        );
    }

@call tdd(mcp_whole_doc_render_honours_the_phase_argument)

### Implementierung

`src/bin/lean_md.rs` — `do_render` (`:39-48`) wird durchreichend:

    fn do_render(
        source: &str,
        jail: std::path::PathBuf,
        consumer: Option<Consumer>,
        crp: Option<CrpMode>,
        phase: Option<&str>,
    ) -> Result<String, lean_md::skills::SkillRenderError> {
        lean_md::render_source_with_phase(source, phase, consumer, crp, jail)
    }

Im `else`-Zweig (`:810-816`) den Aufruf ersetzen:

                                    let phase = args.get("phase").and_then(Value::as_str);
                                    match do_render(&source, jail, consumer, crp, phase) {
                                        Ok(rendered) => rpc_ok(
                                            &id,
                                            json!({ "content": [{ "type": "text", "text": rendered }] }),
                                        ),
                                        // A phase the document does not define is a caller
                                        // error — same verdict as the skill branch above.
                                        Err(e @ lean_md::skills::SkillRenderError::PhaseNotFound(_)) => {
                                            rpc_err(&id, -32602, &format!("{e}"))
                                        }
                                        // Everything else stays a note in the RESULT (#498).
                                        Err(e) => rpc_ok(
                                            &id,
                                            json!({ "content": [{ "type": "text", "text":
                                                format!("<!-- lmd render error: {e:?} -->") }] }),
                                        ),
                                    }

Bestehende `do_render`-Aufrufer mitziehen (`:1017` im Reinheitstest): fünftes
Argument `None`.

### Verify & Close

@call verify(src/bin/lean_md.rs)
@call gate(src/bin/lean_md.rs)
@call commit("src/bin/lean_md.rs", "fix(mcp): honour the phase argument on the path/content render branch")
@call remember_decision("MCP ctx_md_render: phase reaches the path/content branch; PhaseNotFound -> -32602, DuplicatePhase stays a note in the result")
@phase-end

@phase "task-4"
## Task 4 — C2: `jail_root` ist der Projektroot, nicht das Plan-Verzeichnis

**Files:** `src/bin/lean_md.rs`, `docs/lean-md/specs/2026-08-31-lmd-anker-ohne-sitzung-design.md`.

**Interfaces:** neue freie Funktion `project_root_of(&Path) -> Option<PathBuf>`;
`mcp_load_source` liefert diesen Root als Jail.

**Anchors:** `src/bin/lean_md.rs:556-571` (`mcp_load_source`, jailt heute auf
`parent()`) · `src/skills.rs:153-157` (Doc-Kommentar: `jail_root` MUSS der
Projektroot sein — der CLI-Pfad hält das ein, der MCP-Pfad bisher nicht).

**Sicherheitsentscheidung:** der Jail wächst nach oben (Plan-Verzeichnis →
Projektroot). Bewusst, damit CLI- und Gateway-Render auf derselben Wurzel laufen;
wird in der Design-Spec als Entscheidung vermerkt.

### Test zuerst

`src/bin/lean_md.rs`, Test-Modul:

    #[test]
    fn mcp_jails_on_the_project_root_so_project_relative_imports_resolve() {
        let root = std::env::temp_dir().join(format!("lmd_jail_root_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        lean_md::seeds::materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        let plan_dir = root.join("docs/plans");
        std::fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("p.lmd.md");
        std::fs::write(
            &plan,
            "@lean-md\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n@call verify(src/lib.rs)\n",
        )
        .unwrap();

        let (source, jail) = mcp_load_source(&json!({ "path": plan.to_str().unwrap() })).unwrap();
        assert_eq!(jail, root, "the jail must be the project root, not the plan's parent");

        let out = do_render(&source, jail, None, None, None).unwrap();
        assert!(
            !out.contains("fragment not found"),
            "a project-relative @import must resolve: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

@call tdd(mcp_jails_on_the_project_root_so_project_relative_imports_resolve)

### Implementierung

`src/bin/lean_md.rs` — neben `mcp_load_source`:

    /// Walk up from `start` to the first directory carrying a `.lean-ctx/` or `.git/`
    /// marker: that is the project root, the same root the CLI path jails on (cwd). No
    /// marker → `None`, and the caller keeps the file's parent. The jail deliberately
    /// grows upwards (design 2026-08-31 §2.3 C2) so a project-relative
    /// `@import .lean-ctx/lean-md/…` resolves on the MCP path too.
    fn project_root_of(start: &std::path::Path) -> Option<std::path::PathBuf> {
        start
            .ancestors()
            .find(|d| d.join(".lean-ctx").is_dir() || d.join(".git").is_dir())
            .map(std::path::Path::to_path_buf)
    }

Der Jail-Block in `mcp_load_source` (`:566-570`) wird:

        let parent = std::path::Path::new(path).parent().map_or_else(
            || std::path::PathBuf::from("."),
            std::path::Path::to_path_buf,
        );
        let jail = project_root_of(&parent).unwrap_or(parent);
        // `Path::parent` of a bare filename is "" — keep the cwd form the renderer expects.
        let jail = if jail.as_os_str().is_empty() {
            std::path::PathBuf::from(".")
        } else {
            jail
        };
        Ok((source, jail))

Anschließend die Entscheidung in
`docs/lean-md/specs/2026-08-31-lmd-anker-ohne-sitzung-design.md` festhalten — ein
Absatz unter §2.3 C2:

    **Entscheidung (umgesetzt).** Der Gateway-Jail wandert vom Plan-Verzeichnis auf den
    Projektroot (erster Vorfahre mit `.lean-ctx/` oder `.git/`). Er wächst damit nach
    oben; das ist der Preis dafür, daß CLI- und Gateway-Render dieselbe Wurzel sehen und
    `@import .lean-ctx/lean-md/…` überhaupt auflöst. Ohne Marker bleibt es beim
    Elternverzeichnis.

@call patch("docs/lean-md/specs/2026-08-31-lmd-anker-ohne-sitzung-design.md", "the C2 decision paragraph under §2.3")

### Verify & Close

@call verify(src/bin/lean_md.rs docs/lean-md/specs/2026-08-31-lmd-anker-ohne-sitzung-design.md)
@call gate(src/bin/lean_md.rs)
@call commit("src/bin/lean_md.rs docs/lean-md/specs/2026-08-31-lmd-anker-ohne-sitzung-design.md", "fix(mcp): jail the render on the project root so project-relative imports resolve")
@call remember_decision("MCP render jails on the first ancestor with .lean-ctx/ or .git/ — the jail grows upwards on purpose so CLI and gateway share one root")
@phase-end

@phase "task-5"
## Task 5 — C3: Sitzungs-Sinks im MCP-Modus abschalten

**Files:** `src/phases.rs`, `src/bin/lean_md.rs`.

**Interfaces:** Produces `lean_md::phases::disable_session_sinks()` und den
Test-Seam `set_session_sinks_disabled(bool)`.

**Anchors:** `src/phases.rs:214-235` (`session_set_task`, `session_add_finding`,
`session_decision`) · `:117-188` (`fire_action`, die `@on complete`-Verteilung) ·
`:193-212` (`fire_agent`) · `:423-435` (`finalize_phase` ruft `session_decision`
auch am Abbruchpfad, an `fire_action` vorbei) · `src/bin/lean_md.rs:670-702`
(`cmd_mcp`-Vorlauf, wo der Schalter gesetzt wird).

**Warum ein Prozeßschalter:** über den Gateway ist jeder Sink ein `lean-ctx call`
zurück in den lean-ctx-Server, der auf unsere Antwort wartet
(`Gateway → lean-md → lean-ctx call ctx_session → Gateway` → `Transport closed`,
Spec §1.3). Der Schnitt liegt am Prozeßmodus, nicht an einem Konfigurationsknopf.

**Testhinweis:** globaler Zustand — beide Richtungen liegen deshalb in *einer*
Testfunktion, die den Schalter am Ende zurücksetzt (`cargo nextest run` isoliert
zusätzlich pro Prozeß).

### Test zuerst

`src/phases.rs`, Test-Modul (nutzt den `Recorder` aus Task 1 mit `fail: ""`):

    #[test]
    fn mcp_mode_silences_the_session_sinks_and_cli_mode_keeps_them() {
        let src = "@phase \"t1\"\nBODY\n@on complete decision=\"done\"\n@phase-end\n";
        let ctx_for = |calls: &std::rc::Rc<std::cell::RefCell<Vec<String>>>| {
            Rc::new(crate::engine::EngineContext::with_backend(
                crate::header::LeanMdHeader::default(),
                std::path::PathBuf::from("."),
                Box::new(Recorder {
                    calls: calls.clone(),
                    fail: "",
                }),
            ))
        };

        // CLI mode (default): the sink fires.
        let cli = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let _ = render_with_phases(&ctx_for(&cli), src);
        assert!(
            cli.borrow().iter().any(|t| t == "ctx_session"),
            "CLI path must keep its sinks: {:?}",
            cli.borrow()
        );

        // MCP mode: no outbound call at all, body unchanged.
        set_session_sinks_disabled(true);
        let mcp = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let out = render_with_phases(&ctx_for(&mcp), src);
        set_session_sinks_disabled(false); // never leak the switch into another test
        assert!(mcp.borrow().is_empty(), "no sink call in MCP mode: {:?}", mcp.borrow());
        assert!(out.contains("BODY"), "{out}");
    }

@call tdd(mcp_mode_silences_the_session_sinks_and_cli_mode_keeps_them)

### Implementierung

`src/phases.rs` — hinter den `use`-Zeilen (`:8-10`):

    /// Process-wide kill-switch for the outbound session/knowledge/agent sinks.
    /// `lean-md mcp` sets it at start-up: on the gateway path every sink is a
    /// `lean-ctx call` back INTO the lean-ctx server that is waiting for our answer
    /// (`Gateway → lean-md → lean-ctx call ctx_session → Gateway`), which kills the
    /// render with `downstream tools/call failed: Transport closed`. Deliberate
    /// divergence from the CLI path, where the sinks keep firing.
    static SINKS_DISABLED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// Switch the session/knowledge/agent sinks off for this process (MCP server mode).
    pub fn disable_session_sinks() {
        set_session_sinks_disabled(true);
    }

    /// Test seam: flip the switch either way. Production only ever turns it on.
    pub fn set_session_sinks_disabled(disabled: bool) {
        SINKS_DISABLED.store(disabled, std::sync::atomic::Ordering::Relaxed);
    }

    fn sinks_disabled() -> bool {
        SINKS_DISABLED.load(std::sync::atomic::Ordering::Relaxed)
    }

Guard als erste Zeile in `fire_action` (`:117`), `fire_agent` (`:193`),
`session_set_task` (`:214`), `session_add_finding` (`:221`) und `session_decision`
(`:230`) — `fire_action` deckt `remember`/`compress` mit ab, die drei `session_*`
decken den Abbruchpfad über `finalize_phase` ab:

    if sinks_disabled() {
        return;
    }

`src/bin/lean_md.rs` — erste Zeile in `cmd_mcp` (`:670`, vor dem Seed-Refresh):

    // Gateway mode: the sinks would recurse back into the server that is waiting for
    // this render (design 2026-08-31 §2.3 C3).
    lean_md::phases::disable_session_sinks();

### Verify & Close

@call verify(src/phases.rs src/bin/lean_md.rs)
@call review_change()
@call gate(src/)
@call commit("src/phases.rs src/bin/lean_md.rs", "fix(mcp): silence the session sinks in MCP server mode to break the gateway recursion")
@call remember_decision("lean-md mcp disables session/knowledge/agent sinks process-wide (phases::disable_session_sinks) — the CLI path keeps firing them")
@phase-end
