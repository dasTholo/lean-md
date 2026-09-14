@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# `lean-md outline --json` (Binary 0.2.4)

Spec: `docs/lean-md/specs/2026-09-14-lmd-outline-json-design.md`.
Render je Task: `lean-md render docs/lean-md/plans/2026-09-14-lmd-outline-json.lmd.md --phase task-N`.

## Goal

Ein Unterbefehl `lean-md outline <datei|-> --json [--require-phase a,b]` gibt die Gliederung
eines `.lmd.md`-Dokuments als ein JSON-Objekt aus — Phasen mit ihren `@call`s, die Makro-
Signaturen im Geltungsbereich und alle Makro-Befunde —, ohne zu rendern. lean-herdr liest damit
Pläne, statt `split_call_args` in Python nachzubauen.

- **Task 1** — `phases.rs`: `phase_blocks` mit Zeilen und Fence-Markierung; `iter_phase_blocks`
  baut darauf auf.
- **Task 2** — `src/outline.rs`: Phasen, Aufrufe, Makros, JSON; kein Backend-Aufruf.
- **Task 3** — Befunde: `unknown_macro`, `arity`, `malformed_call`, `duplicate_phase`, `import`,
  `missing_phase`.
- **Task 4** — CLI `outline` mit Exit-Codes 0/1/2 und Integrationstests.
- **Task 5** — README, CHANGELOG, Version 0.2.4, Abnahme; Vorbereitungs-Commit.

## Architecture

```
src/phases.rs        + PhaseBlock, phase_blocks; ~ iter_phase_blocks (delegiert);
                       fenced_mask, parse_phase_name, phase_title → pub(crate)                  (Task 1)
src/macros.rs        + MacroRegistry::sorted_defs                             (Task 2)
src/outline.rs       + Outline, OutlinePhase, OutlineCall, OutlineError,
                       outline, outline_with_ctx, Outline::to_json            (Task 2)
                     ~ outline_with_ctx füllt errors                          (Task 3)
src/lib.rs           + pub mod outline                                        (Task 2)
src/bin/lean_md.rs   + parse_outline_flags, cmd_outline; main-Match, Usage    (Task 4)
tests/outline.rs     + Integrationstests                                      (Task 4)
README.md, CHANGELOG.md, Cargo.toml, Cargo.lock, lean-ctx-addon.toml          (Task 5)
```

Anker im Bestand (Textanker, keine aktiven `@read`):

- Phasengrenzen: `src/phases.rs:543-546` (`parse_phase_name`), `:551-581` (`fenced_mask`),
  `:608-625` (`duplicate_phase`), `:647-681` (`iter_phase_blocks`), `:704-720` (`phase_title`).
- Aufrufe: `src/bridges/call.rs` zerlegt mit `parse_call_signature(args.raw())`
  (`src/macros.rs:89-107`); leere Klammern ergeben null Argumente; fehlende Argumente werden beim
  Rendern still `""` (`src/macros.rs:182-195`).
- Makros: `extract_definitions` (`src/macros.rs:306-357`) füllt `ctx.macros`; ein `@import`
  läuft über `import_library` (`:457-470`) → `ctx.fragments.resolve(target, &ctx.jail_root)`;
  `ResolveError` ist `NotFound|Jail|Io` (`src/fragments.rs:32-37`).
- Direktivzeilen: `crate::parser::block::parse_directive_line` (`src/parser/block.rs:14-33`).
- CLI-Muster: `main` (`src/bin/lean_md.rs:238-263`), `cmd_check` (`:396-415`).
- Test-Muster: `tests/list_phases.rs` (Binary + Temp-Verzeichnis), Backend-Recorder
  `src/phases.rs:893-925`.

Ergänzungen zur Spec, alle ohne Folgen für lean-herdr:

- Befundart `malformed_call` für ein `@call`, dessen Signatur `parse_call_signature` ablehnt
  (die Spec kannte nur fünf Arten; das Rendern meldet diesen Fall ebenfalls als Fehler).
- Der Vorbereitungs-Commit hebt zusätzlich `[addon] version` in `lean-ctx-addon.toml`, wie
  `20dc051` (prepare binary 0.2.3); `[artifacts.*]` bleibt unberührt.
- Eine Zeile mit vier oder mehr Leerzeichen Einzug ist ein eingerückter Code-Block und kein
  `@call`. `duplicate_phase` folgt derselben Regel wie `phases::duplicate_phase`: Jede
  `@phase`-Zeile außerhalb von Fenced Code zählt. Die Suche nach verschachtelten Imports
  überspringt `@define`-Rümpfe.
- Die Fälle aus Spec §8 liegen überwiegend als Unit-Tests in `src/outline.rs`;
  `tests/outline.rs` prüft Fenced Code, Komma in Quotes und null Argumente zusätzlich über die CLI.

## Global Constraints

- Non-Goals: kein MCP-Tool für `outline`; keine Änderung an `render`, `check` oder den Seeds;
  kein Rendern ohne Bridges; keine Textausgabe von `outline`.
- `iter_phase_blocks` verhält sich unverändert: bestehende Phasen-, `--list-phases`- und
  `check`-Tests bleiben grün.
- Byte-Stabilität (#498) ist Testgate: `outline` ist eine reine Funktion aus Dokument, Imports
  und Argumenten; Task 2 und Task 4 prüfen zwei Läufe auf Gleichheit.
- `outline` ruft kein Backend, feuert keinen `@on complete`-Sink und schreibt keine Datei
  (Task 2 prüft das mit einem Recorder).
- Keine Seed-Änderung, also kein `LEAN_MD_BLESS`.
- Reihenfolge 1 → 2 → 3 → 4 → 5; jede Task baut auf der vorigen auf.
- Die Arbeit endet mit dem Vorbereitungs-Commit (Task 5): kein Tag, kein Push.

@phase "task-1"
## Task 1: `phase_blocks` — Phasen mit Zeilen, auch doppelte

**Files:** Modify `src/phases.rs`.

**Interfaces — Produces** (`src/phases.rs`):

    #[derive(Debug, Clone, PartialEq)]
    pub struct PhaseBlock {
        pub name: String,
        pub line: usize,
        pub body: Vec<(usize, String, bool)>,
    }
    pub fn phase_blocks(source: &str) -> Vec<PhaseBlock>
    pub(crate) fn fenced_mask(source: &str) -> Vec<bool>      // bisher privat
    pub(crate) fn phase_title(body: &str) -> String            // bisher privat
    pub(crate) fn parse_phase_name(rest: &str) -> String       // bisher privat

**Consumes:** `parse_phase_name`, `fenced_mask`, `duplicate_phase` (`src/phases.rs:543-625`).

### Schritt 1 — Tests zuerst

Im Test-Modul von `src/phases.rs`, direkt nach `iter_phase_blocks_orders_phases`:

    #[test]
    fn phase_blocks_keep_duplicates_with_their_lines() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        let blocks = super::phase_blocks(src);
        assert_eq!(blocks.len(), 2);
        assert_eq!((blocks[0].name.as_str(), blocks[0].line), ("t", 4));
        assert_eq!((blocks[1].name.as_str(), blocks[1].line), ("t", 7));
        assert_eq!(blocks[1].body, vec![(8, "second".to_string(), false)]);
    }

    #[test]
    fn phase_blocks_mark_fenced_body_lines() {
        let src = "@phase \"a\"\n```\n@call x() /\n```\n@call y() /\n@phase-end\n";
        let blocks = super::phase_blocks(src);
        let flags: Vec<(usize, bool)> = blocks[0].body.iter().map(|(n, _, f)| (*n, *f)).collect();
        assert_eq!(flags, vec![(2, true), (3, true), (4, true), (5, false)]);
    }

    #[test]
    fn phase_blocks_drop_an_unterminated_phase() {
        let blocks = super::phase_blocks("@phase \"a\"\nbody\n");
        assert!(blocks.is_empty());
    }

@call test(phase_blocks)

Expected: FAIL — `cannot find function phase_blocks`.

### Schritt 2 — Implementierung

@call patch("src/phases.rs", "parse_phase_name (fn line 544), fenced_mask (fn line 551), iter_phase_blocks (lines 643-681), phase_title (fn line 704)")

- `fn parse_phase_name` → `pub(crate) fn parse_phase_name`; `fn fenced_mask` →
  `pub(crate) fn fenced_mask`; `fn phase_title` → `pub(crate) fn phase_title`.
- Direkt vor dem Doc-Kommentar von `iter_phase_blocks` einfügen:

      /// One `@phase "name" … @phase-end` block with its position in `source`. Unlike
      /// `iter_phase_blocks`, a duplicated name keeps every block: `outline` reports the
      /// duplicate instead of hiding it. `body` holds `(1-based line, text, fenced)`.
      #[derive(Debug, Clone, PartialEq)]
      pub struct PhaseBlock {
          pub name: String,
          /// 1-based line of the `@phase` directive.
          pub line: usize,
          pub body: Vec<(usize, String, bool)>,
      }

      /// Every complete phase block in document order, duplicates included. Same boundary
      /// rules as `iter_phase_blocks`: flat v1 phases, a nested `@phase` line is ignored,
      /// fenced lines never open or close a phase, an unterminated block is dropped.
      pub fn phase_blocks(source: &str) -> Vec<PhaseBlock> {
          let fenced = fenced_mask(source);
          let mut out = Vec::new();
          let mut open: Option<PhaseBlock> = None;
          for (idx, line) in source.lines().enumerate() {
              let trimmed = line.trim_start();
              if !fenced[idx] && trimmed.starts_with("@phase-end") {
                  if let Some(block) = open.take() {
                      out.push(block);
                  }
                  continue;
              }
              if !fenced[idx]
                  && let Some(rest) = trimmed.strip_prefix("@phase")
              {
                  if open.is_none() {
                      open = Some(PhaseBlock {
                          name: parse_phase_name(rest),
                          line: idx + 1,
                          body: Vec::new(),
                      });
                  }
                  continue;
              }
              if let Some(block) = open.as_mut() {
                  block.body.push((idx + 1, line.to_string(), fenced[idx]));
              }
          }
          out
      }

- Den Rumpf von `iter_phase_blocks` (nach dem Doc-Kommentar) ersetzen durch:

      pub(crate) fn iter_phase_blocks(source: &str) -> Vec<(String, String)> {
          // A lossy source has no correct block list: the second block of a duplicated name
          // would silently drop out here. Refuse instead — `outline_phases` (→ --list-phases)
          // and `capture_phase_bodies` (→ --phase) both read this, so neither surface can
          // present one of two blocks as if it were the whole file.
          if duplicate_phase(source).is_some() {
              return Vec::new();
          }
          phase_blocks(source)
              .into_iter()
              .map(|block| {
                  let lines: Vec<&str> = block.body.iter().map(|(_, text, _)| text.as_str()).collect();
                  (block.name, lines.join("\n"))
              })
              .collect()
      }

@call test(phase)

Expected: PASS — die drei neuen Tests und alle bestehenden `phase`-Tests (u. a.
`iter_phase_blocks_orders_phases`, `outline_*`, `duplicate_phase_*`).

@call verify(src/phases.rs)
@call gate(src/phases.rs)
@call commit("src/phases.rs", "feat(phases): expose phase blocks with source lines, duplicates included")
@phase-end

@phase "task-2"
## Task 2: `src/outline.rs` — Phasen, Aufrufe, Makros als JSON

**Files:** Create `src/outline.rs`. Modify `src/lib.rs`, `src/macros.rs`.

**Interfaces — Produces:**

    // src/macros.rs
    impl MacroRegistry { pub fn sorted_defs(&self) -> Vec<&MacroDef> }
    // src/outline.rs
    pub struct OutlineCall { pub macro_name: String, pub args: Vec<String>, pub line: usize }
    pub struct OutlinePhase { pub name: String, pub title: String, pub line: usize, pub calls: Vec<OutlineCall> }
    pub struct OutlineError { pub kind: &'static str, pub line: usize, pub phase: Option<String>, pub message: String }
    pub struct Outline { pub phases: Vec<OutlinePhase>, pub macros: BTreeMap<String, Vec<String>>, pub errors: Vec<OutlineError> }
    pub fn outline(source: &str, jail_root: PathBuf, required: &[String]) -> Outline
    pub fn outline_with_ctx(ctx: &Rc<EngineContext>, source: &str, required: &[String]) -> Outline
    impl Outline { pub fn to_json(&self) -> serde_json::Value }

**Consumes:** `phase_blocks`, `phase_title` (Task 1); `extract_definitions`,
`parse_call_signature`; `parse_directive_line`; `EngineContext::new` / `with_backend`
(`src/engine.rs:61-104`).

### Schritt 1 — `sorted_defs`, Test zuerst

Im Test-Modul von `src/macros.rs`, nach `macro_signatures_extract`:

    #[test]
    fn sorted_defs_lists_every_macro_by_name() {
        let mut reg = MacroRegistry::new();
        for n in ["b", "a"] {
            reg.insert_authored(MacroDef {
                name: n.to_string(),
                params: vec!["p".to_string()],
                body: String::new(),
            });
        }
        let names: Vec<&str> = reg.sorted_defs().iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
    }

In `impl MacroRegistry`, nach `is_empty`:

    /// Every authored macro sorted by name — the order `outline` reports them in (#498).
    pub fn sorted_defs(&self) -> Vec<&MacroDef> {
        let mut names: Vec<&String> = self.authored.keys().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| self.authored.get(name))
            .collect()
    }

@call test(sorted_defs)

Expected: PASS.

### Schritt 2 — Tests für `outline` zuerst

`src/outline.rs` beginnt mit dem Test-Modul (Produktionscode folgt in Schritt 3 darüber):

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::backend::{BackendError, CodeIntelBackend};
        use crate::header::LeanMdHeader;
        use std::cell::RefCell;

        const PLAN: &str = "@lean-md\nconsumer: ai\n\n@define route(work, lane, files)\n<!-- route -->\n@define-end\n\n@phase \"task-1\"\n@call route(implement, core, \"a.py b.py\")\n## Task 1: first\n```\n@call route(x) /\n```\n@phase-end\n";

        fn run(src: &str) -> Outline {
            outline(src, std::env::temp_dir(), &[])
        }

        #[test]
        fn outline_lists_phases_calls_and_macros() {
            let o = run(PLAN);
            assert_eq!(o.phases.len(), 1);
            let p = &o.phases[0];
            assert_eq!((p.name.as_str(), p.title.as_str(), p.line), ("task-1", "Task 1: first", 8));
            assert_eq!(
                p.calls,
                vec![OutlineCall {
                    macro_name: "route".to_string(),
                    args: vec!["implement".to_string(), "core".to_string(), "a.py b.py".to_string()],
                    line: 9,
                }]
            );
            assert_eq!(
                o.macros.get("route"),
                Some(&vec!["work".to_string(), "lane".to_string(), "files".to_string()])
            );
        }

        #[test]
        fn empty_parens_are_zero_arguments() {
            let o = run("@phase \"a\"\n@call g() /\n@phase-end\n");
            assert!(o.phases[0].calls[0].args.is_empty());
        }

        #[test]
        fn indented_code_lines_are_not_calls() {
            let o = run("@phase \"a\"\n    @call g() /\n@phase-end\n");
            assert!(o.phases[0].calls.is_empty());
        }

        #[test]
        fn quoted_commas_stay_in_one_argument() {
            let o = run("@phase \"a\"\n@call g(\"x, y\", z) /\n@phase-end\n");
            assert_eq!(o.phases[0].calls[0].args, vec!["x, y".to_string(), "z".to_string()]);
        }

        #[test]
        fn a_later_define_wins_in_macros() {
            let o = run("@define g(a)\nx\n@define-end\n@define g(a, b)\ny\n@define-end\n");
            assert_eq!(o.macros.get("g"), Some(&vec!["a".to_string(), "b".to_string()]));
        }

        struct Recorder(Rc<RefCell<Vec<String>>>);
        impl CodeIntelBackend for Recorder {
            fn call(&self, tool: &str, _args: serde_json::Value) -> Result<String, BackendError> {
                self.0.borrow_mut().push(tool.to_string());
                Ok(String::new())
            }
        }

        #[test]
        fn outline_calls_no_backend_and_fires_no_sink() {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let ctx = Rc::new(EngineContext::with_backend(
                LeanMdHeader::default(),
                std::env::temp_dir(),
                Box::new(Recorder(calls.clone())),
            ));
            let src = "@phase \"t\"\n@read src/lib.rs mode=full\n@on complete decision=\"t done\"\n@phase-end\n";
            let _ = outline_with_ctx(&ctx, src, &[]);
            assert!(calls.borrow().is_empty(), "outline must not reach the backend: {:?}", calls.borrow());
        }

        #[test]
        fn json_carries_the_three_top_level_keys_and_is_byte_stable() {
            let v = run(PLAN).to_json();
            assert_eq!(v["phases"][0]["calls"][0]["macro"], "route");
            assert_eq!(v["macros"]["route"], serde_json::json!(["work", "lane", "files"]));
            assert_eq!(v["errors"], serde_json::json!([]));
            assert_eq!(run(PLAN).to_json().to_string(), run(PLAN).to_json().to_string());
        }
    }

In `src/lib.rs`, nach `pub mod node;`: `pub mod outline;`

@call test(outline)

Expected: FAIL — `cannot find function outline` / Typen fehlen.

### Schritt 3 — Implementierung

Über dem Test-Modul in `src/outline.rs`:

    //! `lean-md outline`: the structure of an `.lmd.md` document as data — phases, the
    //! `@call`s inside them, the macro signatures in scope and every macro finding —
    //! without rendering. No bridge runs, no session sink fires, no file is written:
    //! a pure function of the source, its imports and the arguments (#498).

    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::rc::Rc;

    use serde_json::{Map, Value, json};

    use crate::engine::EngineContext;
    use crate::header::parse_header;
    use crate::macros::{extract_definitions, parse_call_signature};
    use crate::parser::block::parse_directive_line;
    use crate::phases::{phase_blocks, phase_title};

    /// One active `@call` line: the macro, its arguments split exactly as a render splits
    /// them (no padding, no truncation), and its 1-based line.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OutlineCall {
        pub macro_name: String,
        pub args: Vec<String>,
        pub line: usize,
    }

    /// One phase in document order.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OutlinePhase {
        pub name: String,
        pub title: String,
        pub line: usize,
        pub calls: Vec<OutlineCall>,
    }

    /// One finding. `phase` is `None` when the line belongs to no phase.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OutlineError {
        pub kind: &'static str,
        pub line: usize,
        pub phase: Option<String>,
        pub message: String,
    }

    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct Outline {
        pub phases: Vec<OutlinePhase>,
        pub macros: BTreeMap<String, Vec<String>>,
        pub errors: Vec<OutlineError>,
    }

    /// Outline of `source` with imports resolved against `jail_root`.
    pub fn outline(source: &str, jail_root: PathBuf, required: &[String]) -> Outline {
        let (header, _) = parse_header(source);
        let ctx = Rc::new(EngineContext::new(header, jail_root));
        outline_with_ctx(&ctx, source, required)
    }

    /// Same as [`outline`] on a caller-built context (tests inject a recording backend).
    pub fn outline_with_ctx(ctx: &Rc<EngineContext>, source: &str, required: &[String]) -> Outline {
        let (_, body) = parse_header(source);
        let _ = extract_definitions(ctx, body);
        let macros = ctx
            .macros
            .borrow()
            .sorted_defs()
            .into_iter()
            .map(|def| (def.name.clone(), def.params.clone()))
            .collect();
        let phases = phase_blocks(source)
            .into_iter()
            .map(|block| {
                let text: Vec<&str> = block.body.iter().map(|(_, l, _)| l.as_str()).collect();
                let calls = block
                    .body
                    .iter()
                    .filter(|(_, _, fenced)| !fenced)
                    .filter_map(|(line, text, _)| call_at(*line, text).and_then(Result::ok))
                    .collect();
                OutlinePhase {
                    name: block.name,
                    title: phase_title(&text.join("\n")),
                    line: block.line,
                    calls,
                }
            })
            .collect();
        let _ = required;
        Outline {
            phases,
            macros,
            errors: Vec::new(),
        }
    }

    /// `Some(Ok(call))` for a well-formed `@call` line, `Some(Err(()))` for a malformed one,
    /// `None` for any other line, and for a line indented four spaces or more (an indented
    /// code block, never a directive).
    fn call_at(line: usize, text: &str) -> Option<Result<OutlineCall, ()>> {
        let trimmed = text.trim_start();
        if text.len() - trimmed.len() >= 4 {
            return None;
        }
        let (name, args) = parse_directive_line(trimmed.as_bytes())?;
        if name != "call" {
            return None;
        }
        Some(
            parse_call_signature(&args)
                .map(|(macro_name, args)| OutlineCall {
                    macro_name,
                    args,
                    line,
                })
                .ok_or(()),
        )
    }

    impl Outline {
        /// The CLI payload. Object keys follow `serde_json`'s map order, lists keep document
        /// order — either way two runs over the same input are byte-identical.
        pub fn to_json(&self) -> Value {
            let phases: Vec<Value> = self
                .phases
                .iter()
                .map(|p| {
                    let calls: Vec<Value> = p
                        .calls
                        .iter()
                        .map(|c| json!({"macro": c.macro_name, "args": c.args, "line": c.line}))
                        .collect();
                    json!({"name": p.name, "title": p.title, "line": p.line, "calls": calls})
                })
                .collect();
            let errors: Vec<Value> = self
                .errors
                .iter()
                .map(|e| {
                    let mut o = Map::new();
                    o.insert("kind".to_string(), json!(e.kind));
                    o.insert("line".to_string(), json!(e.line));
                    if let Some(phase) = &e.phase {
                        o.insert("phase".to_string(), json!(phase));
                    }
                    o.insert("message".to_string(), json!(e.message));
                    Value::Object(o)
                })
                .collect();
            json!({"phases": phases, "macros": self.macros, "errors": errors})
        }
    }

@call test(outline)

Expected: PASS — `outline_lists_phases_calls_and_macros`, `empty_parens_are_zero_arguments`,
`quoted_commas_stay_in_one_argument`, `a_later_define_wins_in_macros`, `indented_code_lines_are_not_calls`,
`outline_calls_no_backend_and_fires_no_sink`, `json_carries_the_three_top_level_keys_and_is_byte_stable`.

@call verify(src/outline.rs src/lib.rs src/macros.rs)
@call gate(src/outline.rs src/lib.rs src/macros.rs)
@call commit("src/outline.rs src/lib.rs src/macros.rs", "feat(outline): report phases, calls and macro signatures as data")
@phase-end

@phase "task-3"
## Task 3: Befunde in `outline`

**Files:** Modify `src/outline.rs`.

**Interfaces — Produces:** `Outline.errors`, sortiert nach `(line, kind)`, mit diesen Arten und
exakt diesen Meldungen:

| `kind` | `line` | `phase` | `message` |
|---|---|---|---|
| `unknown_macro` | Zeile des `@call` | umgebende Phase | `macro not found: <name>` |
| `arity` | Zeile des `@call` | umgebende Phase | `<name> takes <p> argument(s), got <a>` |
| `malformed_call` | Zeile des `@call` | umgebende Phase | `malformed @call signature` |
| `duplicate_phase` | Zeile des zweiten `@phase` | Name | `duplicate @phase "<name>" — first defined at line <first>, again at line <dup>` |
| `import` | Zeile des `@import` im Dokument | — | `@import <target> failed: <ResolveError Debug>` |
| `missing_phase` | `0` | Name | `required @phase "<name>" is missing` |

Geprüft wird jedes aktive `@call` im Dokument außerhalb von Fenced Code und außerhalb von
`@define … @define-end`. Ein fehlschlagender verschachtelter Import wird der Zeile des
`@import` im Dokument zugeordnet.

**Consumes:** `phase_blocks`, `fenced_mask` (Task 1); `ctx.fragments.resolve`
(`src/fragments.rs:96-102`).

### Schritt 1 — Tests zuerst

Im Test-Modul von `src/outline.rs`:

    fn err(kind: &'static str, line: usize, phase: Option<&str>, message: &str) -> OutlineError {
        OutlineError {
            kind,
            line,
            phase: phase.map(str::to_string),
            message: message.to_string(),
        }
    }

    #[test]
    fn unknown_macro_is_reported_with_its_phase() {
        let o = run("@phase \"task-1\"\n@call nope(a) /\n@phase-end\n");
        assert_eq!(o.errors, vec![err("unknown_macro", 2, Some("task-1"), "macro not found: nope")]);
    }

    #[test]
    fn arity_mismatch_is_reported() {
        let o = run("@define g(a, b)\nx\n@define-end\n@call g(1) /\n");
        assert_eq!(o.errors, vec![err("arity", 4, None, "g takes 2 argument(s), got 1")]);
    }

    #[test]
    fn malformed_call_is_reported() {
        let o = run("@call broken /\n");
        assert_eq!(o.errors, vec![err("malformed_call", 1, None, "malformed @call signature")]);
    }

    #[test]
    fn duplicate_phase_is_reported_at_the_second_site() {
        let o = run("@phase \"t\"\na\n@phase-end\n@phase \"t\"\nb\n@phase-end\n");
        assert_eq!(o.phases.len(), 2);
        assert_eq!(
            o.errors,
            vec![err(
                "duplicate_phase",
                4,
                Some("t"),
                "duplicate @phase \"t\" — first defined at line 1, again at line 4"
            )]
        );
    }

    #[test]
    fn a_missing_import_is_reported() {
        let dir = std::env::temp_dir().join(format!("lmd_outline_import_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let o = outline("@import .lean-ctx/lean-md/nope /\n", dir.clone(), &[]);
        assert_eq!(o.errors.len(), 1);
        assert_eq!((o.errors[0].kind, o.errors[0].line), ("import", 1));
        assert!(o.errors[0].message.starts_with("@import .lean-ctx/lean-md/nope failed"), "{:?}", o.errors);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_required_phase_is_reported() {
        let o = outline("@phase \"a\"\n@phase-end\n", std::env::temp_dir(), &["a".to_string(), "lanes".to_string()]);
        assert_eq!(o.errors, vec![err("missing_phase", 0, Some("lanes"), "required @phase \"lanes\" is missing")]);
    }

    #[test]
    fn calls_in_fences_and_define_bodies_are_not_checked() {
        let o = run("```\n@call nope() /\n```\n@define w()\n@call nope2() /\n@define-end\n");
        assert!(o.errors.is_empty(), "{:?}", o.errors);
    }

    #[test]
    fn errors_are_sorted_by_line_then_kind() {
        let o = outline("@call b() /\n@call a(1) /\n", std::env::temp_dir(), &["x".to_string()]);
        let order: Vec<(usize, &str)> = o.errors.iter().map(|e| (e.line, e.kind)).collect();
        assert_eq!(order, vec![(0, "missing_phase"), (1, "unknown_macro"), (2, "unknown_macro")]);
    }

@call test(outline)

Expected: FAIL — `errors` ist leer.

### Schritt 2 — Implementierung

@call patch("src/outline.rs", "the use line of crate::phases and the whole fn outline_with_ctx")

- `use crate::phases::{phase_blocks, phase_title};` →
  `use crate::phases::{PhaseBlock, fenced_mask, parse_phase_name, phase_blocks, phase_title};`
- `outline_with_ctx` (Doc-Kommentar bleibt) vollständig ersetzen durch:

      pub fn outline_with_ctx(ctx: &Rc<EngineContext>, source: &str, required: &[String]) -> Outline {
          let (_, body) = parse_header(source);
          let _ = extract_definitions(ctx, body);
          let macros = ctx
              .macros
              .borrow()
              .sorted_defs()
              .into_iter()
              .map(|def| (def.name.clone(), def.params.clone()))
              .collect();
          let blocks = phase_blocks(source);
          let phases = blocks
              .iter()
              .map(|block| {
                  let text: Vec<&str> = block.body.iter().map(|(_, l, _)| l.as_str()).collect();
                  let calls = block
                      .body
                      .iter()
                      .filter(|(_, _, fenced)| !fenced)
                      .filter_map(|(line, text, _)| call_at(*line, text).and_then(Result::ok))
                      .collect();
                  OutlinePhase {
                      name: block.name.clone(),
                      title: phase_title(&text.join("\n")),
                      line: block.line,
                      calls,
                  }
              })
              .collect();
          let errors = findings(ctx, source, &blocks, required);
          Outline {
              phases,
              macros,
              errors,
          }
      }
- Unter `call_at` einfügen:

      /// Every finding in `source`, sorted by `(line, kind)`.
      fn findings(
          ctx: &Rc<EngineContext>,
          source: &str,
          blocks: &[PhaseBlock],
          required: &[String],
      ) -> Vec<OutlineError> {
          let mut out = Vec::new();
          let fenced = fenced_mask(source);
          let mut in_define = false;
          let mut seen_imports: Vec<String> = Vec::new();
          for (idx, text) in source.lines().enumerate() {
              let line = idx + 1;
              let trimmed = text.trim_start();
              if fenced[idx] {
                  continue;
              }
              if trimmed.starts_with("@define-end") {
                  in_define = false;
                  continue;
              }
              if trimmed.starts_with("@define") {
                  in_define = true;
                  continue;
              }
              if in_define {
                  continue;
              }
              if let Some(rest) = trimmed.strip_prefix("@import") {
                  let target = rest.trim().trim_end_matches('/').trim();
                  if !target.is_empty() {
                      import_errors(ctx, target, line, &mut seen_imports, &mut out);
                  }
                  continue;
              }
              let phase = phase_of(blocks, line);
              match call_at(line, text) {
                  None => {}
                  Some(Err(())) => out.push(OutlineError {
                      kind: "malformed_call",
                      line,
                      phase,
                      message: "malformed @call signature".to_string(),
                  }),
                  Some(Ok(call)) => match ctx.macros.borrow().get(&call.macro_name) {
                      None => out.push(OutlineError {
                          kind: "unknown_macro",
                          line,
                          phase,
                          message: format!("macro not found: {}", call.macro_name),
                      }),
                      Some(def) if def.params.len() != call.args.len() => out.push(OutlineError {
                          kind: "arity",
                          line,
                          phase,
                          message: format!(
                              "{} takes {} argument(s), got {}",
                              call.macro_name,
                              def.params.len(),
                              call.args.len()
                          ),
                      }),
                      Some(_) => {}
                  },
              }
          }
          // Same rule as `phases::duplicate_phase`, which makes `render` and `check` refuse the
          // source: every `@phase` line outside a fence counts, nested or unterminated ones too.
          let mut first_seen: Vec<(String, usize)> = Vec::new();
          for (idx, text) in source.lines().enumerate() {
              let trimmed = text.trim_start();
              if fenced[idx] || trimmed.starts_with("@phase-end") {
                  continue;
              }
              let Some(rest) = trimmed.strip_prefix("@phase") else {
                  continue;
              };
              let name = parse_phase_name(rest);
              match first_seen.iter().find(|(seen, _)| *seen == name) {
                  Some((_, first)) => out.push(OutlineError {
                      kind: "duplicate_phase",
                      line: idx + 1,
                      phase: Some(name.clone()),
                      message: format!(
                          "duplicate @phase \"{name}\" — first defined at line {first}, again at line {}",
                          idx + 1
                      ),
                  }),
                  None => first_seen.push((name, idx + 1)),
              }
          }
          for name in required {
              if !blocks.iter().any(|block| block.name == *name) {
                  out.push(OutlineError {
                      kind: "missing_phase",
                      line: 0,
                      phase: Some(name.clone()),
                      message: format!("required @phase \"{name}\" is missing"),
                  });
              }
          }
          out.sort_by(|a, b| (a.line, a.kind).cmp(&(b.line, b.kind)));
          out
      }

      /// The phase whose body holds `line`, if any.
      fn phase_of(blocks: &[PhaseBlock], line: usize) -> Option<String> {
          blocks
              .iter()
              .find(|block| {
                  let end = block.body.last().map_or(block.line, |(n, _, _)| *n);
                  block.line < line && line <= end
              })
              .map(|block| block.name.clone())
      }

      /// Resolve `target` like `@import` does and follow its own imports; every failure is
      /// reported at `line`, the `@import` in the outlined document.
      fn import_errors(
          ctx: &Rc<EngineContext>,
          target: &str,
          line: usize,
          seen: &mut Vec<String>,
          out: &mut Vec<OutlineError>,
      ) {
          if seen.iter().any(|t| t == target) {
              return;
          }
          seen.push(target.to_string());
          match ctx.fragments.resolve(target, &ctx.jail_root) {
              Ok(content) => {
                  let mut in_define = false;
                  for text in content.lines() {
                      let trimmed = text.trim_start();
                      if trimmed.starts_with("@define-end") {
                          in_define = false;
                          continue;
                      }
                      if trimmed.starts_with("@define") {
                          in_define = true;
                          continue;
                      }
                      if in_define {
                          continue;
                      }
                      if let Some(rest) = trimmed.strip_prefix("@import") {
                          let nested = rest.trim().trim_end_matches('/').trim();
                          if !nested.is_empty() {
                              import_errors(ctx, nested, line, seen, out);
                          }
                      }
                  }
              }
              Err(e) => out.push(OutlineError {
                  kind: "import",
                  line,
                  phase: None,
                  message: format!("@import {target} failed: {e:?}"),
              }),
          }
      }

@call test(outline)

Expected: PASS — alle `outline`-Tests aus Task 2 und Task 3.

@call verify(src/outline.rs)
@call gate(src/outline.rs)
@call commit("src/outline.rs", "feat(outline): report unknown macros, arity, malformed calls, duplicate phases, imports and missing phases")
@phase-end

@phase "task-4"
## Task 4: CLI `lean-md outline`

**Files:** Modify `src/bin/lean_md.rs`. Create `tests/outline.rs`.

**Interfaces — Produces** (CLI):

    lean-md outline <file.lmd.md | -> --json [--require-phase <name>[,<name>…]]
    stdout: one JSON object (Outline::to_json)   exit 0: no findings   exit 1: findings
    exit 2: usage error or unreadable input — message on stderr, nothing on stdout

**Consumes:** `lean_md::outline::outline` (Task 2/3).

### Schritt 1 — Integrationstests zuerst

`tests/outline.rs`:

    //! `outline --json`: phases, @calls, macro signatures and findings as data.
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn run(args: &[&str], cwd: &std::path::Path, stdin: Option<&str>) -> (String, String, i32) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lean-md"))
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn lean-md");
        if let Some(text) = stdin {
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(text.as_bytes())
                .expect("write stdin");
        }
        drop(child.stdin.take());
        let out = child.wait_with_output().expect("wait lean-md");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code().unwrap_or(-1),
        )
    }

    fn project(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lmd_outline_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".lean-ctx/lean-md")).unwrap();
        std::fs::write(
            dir.join(".lean-ctx/lean-md/recipes.lmd.md"),
            "@define route(work, lane, files)\n<!-- route -->\n@define-end\n",
        )
        .unwrap();
        dir
    }

    const PLAN: &str = "@lean-md\nconsumer: ai\n\n@import .lean-ctx/lean-md/recipes /\n\n@phase \"constraints\"\n## Global Constraints\n@phase-end\n@phase \"task-1\"\n@call route(implement, core, \"a.py b.py\")\n## Task 1: first\n@phase-end\n";

    #[test]
    fn a_clean_plan_exits_zero_with_json() {
        let dir = project("clean");
        std::fs::write(dir.join("p.lmd.md"), PLAN).unwrap();
        let (stdout, stderr, code) = run(
            &["outline", "p.lmd.md", "--json", "--require-phase", "constraints"],
            &dir,
            None,
        );
        assert_eq!(code, 0, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("json on stdout");
        assert_eq!(v["errors"], serde_json::json!([]));
        assert_eq!(v["phases"][1]["name"], "task-1");
        assert_eq!(v["phases"][1]["calls"][0]["args"], serde_json::json!(["implement", "core", "a.py b.py"]));
        assert_eq!(v["macros"]["route"], serde_json::json!(["work", "lane", "files"]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stdin_gives_the_same_bytes_as_the_file() {
        let dir = project("stdin");
        std::fs::write(dir.join("p.lmd.md"), PLAN).unwrap();
        let (from_file, _, _) = run(&["outline", "p.lmd.md", "--json"], &dir, None);
        let (from_stdin, stderr, code) = run(&["outline", "-", "--json"], &dir, Some(PLAN));
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(from_stdin, from_file);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn findings_exit_one_and_still_print_json() {
        let dir = project("findings");
        let (stdout, _, code) = run(
            &["outline", "-", "--json", "--require-phase", "lanes"],
            &dir,
            Some("@phase \"task-1\"\n@call nope() /\n@phase-end\n"),
        );
        assert_eq!(code, 1);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("json on stdout");
        let kinds: Vec<&str> = v["errors"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["missing_phase", "unknown_macro"]);
        assert!(v["errors"][0].get("phase").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fenced_quoted_and_empty_calls_survive_the_cli() {
        let dir = project("forms");
        let src = "@define g(a, b)\nx\n@define-end\n@define h()\ny\n@define-end\n@phase \"t\"\n```\n@call nope() /\n```\n@call g(\"x, y\", z) /\n@call h() /\n@phase-end\n";
        let (stdout, stderr, code) = run(&["outline", "-", "--json"], &dir, Some(src));
        assert_eq!(code, 0, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("json on stdout");
        let calls = &v["phases"][0]["calls"];
        assert_eq!(calls.as_array().unwrap().len(), 2);
        assert_eq!(calls[0]["args"], serde_json::json!(["x, y", "z"]));
        assert_eq!(calls[1]["args"], serde_json::json!([]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_runs_are_byte_identical() {
        let dir = project("stable");
        std::fs::write(dir.join("p.lmd.md"), PLAN).unwrap();
        let (a, _, _) = run(&["outline", "p.lmd.md", "--json"], &dir, None);
        let (b, _, _) = run(&["outline", "p.lmd.md", "--json"], &dir, None);
        assert_eq!(a, b);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn usage_errors_exit_two_without_json() {
        let dir = project("usage");
        for args in [
            vec!["outline", "p.lmd.md"],
            vec!["outline", "--json"],
            vec!["outline", "missing.lmd.md", "--json"],
            vec!["outline", "p.lmd.md", "--json", "--bogus"],
            vec!["outline", "p.lmd.md", "--json", "--require-phase"],
        ] {
            let (stdout, stderr, code) = run(&args, &dir, None);
            assert_eq!(code, 2, "{args:?}: {stderr}");
            assert_eq!(stdout, "", "{args:?}");
            assert!(stderr.starts_with("lean-md outline: "), "{args:?}: {stderr}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

Run: {{ var test_cmd }} --test outline

Expected: FAIL — der Unterbefehl `outline` endet heute in der Usage-Meldung mit Exit 1.

### Schritt 2 — Implementierung

@call patch("src/bin/lean_md.rs", "the module doc subcommand list (lines 7-10), the main match (lines 241-261) and the line before `// ─── ack subcommand`")

- Modul-Doc, nach der `check`-Zeile: `//!   outline <file|-> --json [--require-phase a,b]`
- `main`: nach `"check" => cmd_check(&args[1..]),` die Zeile `"outline" => cmd_outline(&args[1..]),`;
  im Usage-Text `<render|check|mcp|skill|source|ack>` → `<render|check|outline|mcp|skill|source|ack>`
  und nach der `check`-Zeile:
  `\n  outline <file.lmd.md|-> --json [--require-phase a,b]  (structure + findings as JSON; exit 1 on findings)\`
- Vor `// ─── ack subcommand` einfügen:

      // ─── outline subcommand ────────────────────────────────────────────────────

      /// `(file, required phases)` from the `outline` arguments. `Err` is a usage error:
      /// exit 2, message on stderr, nothing on stdout.
      fn parse_outline_flags(rest: &[String]) -> Result<(String, Vec<String>), String> {
          let mut file: Option<String> = None;
          let mut json = false;
          let mut required = Vec::new();
          let mut i = 0;
          while i < rest.len() {
              match rest[i].as_str() {
                  "--json" => json = true,
                  "--require-phase" => {
                      i += 1;
                      let Some(list) = rest.get(i) else {
                          return Err("--require-phase needs a comma-separated list".to_string());
                      };
                      required.extend(
                          list.split(',')
                              .map(str::trim)
                              .filter(|name| !name.is_empty())
                              .map(str::to_string),
                      );
                  }
                  "-" if file.is_none() => file = Some("-".to_string()),
                  arg if arg.starts_with('-') => return Err(format!("unknown flag {arg}")),
                  arg if file.is_none() => file = Some(arg.to_string()),
                  arg => return Err(format!("unexpected argument {arg}")),
              }
              i += 1;
          }
          let Some(file) = file else {
              return Err("missing <file.lmd.md> or -".to_string());
          };
          if !json {
              return Err("--json is required".to_string());
          }
          Ok((file, required))
      }

      fn cmd_outline(rest: &[String]) {
          let (file, required) = match parse_outline_flags(rest) {
              Ok(parsed) => parsed,
              Err(e) => {
                  eprintln!("lean-md outline: {e}");
                  std::process::exit(2);
              }
          };
          let read = if file == "-" {
              std::io::read_to_string(std::io::stdin())
          } else {
              std::fs::read_to_string(&file)
          };
          let source = match read {
              Ok(source) => source,
              Err(e) => {
                  eprintln!("lean-md outline: read {file}: {e}");
                  std::process::exit(2);
              }
          };
          let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
          let outline = lean_md::outline::outline(&source, jail, &required);
          println!("{}", outline.to_json());
          if !outline.errors.is_empty() {
              std::process::exit(1);
          }
      }

Run: {{ var test_cmd }} --test outline

Expected: PASS — alle sechs Tests in `tests/outline.rs`.

@call test(outline)

Expected: PASS — die `outline`-Unit-Tests aus Task 2 und Task 3.

@call verify(src/bin/lean_md.rs tests/outline.rs)
@call review_change()
@call gate(src/bin/lean_md.rs tests/outline.rs)
@call commit("src/bin/lean_md.rs tests/outline.rs", "feat(cli): add outline --json with exit codes 0/1/2")
@phase-end

@phase "task-5"
## Task 5: Doku, Version 0.2.4, Abnahme, Vorbereitungs-Commit

**Files:** Modify `README.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, `lean-ctx-addon.toml`.

**Interfaces:** keine Code-Schnittstelle. **Produces:** Binary-Version 0.2.4 in allen drei
Versionsdateien; README und CHANGELOG beschreiben `outline`.

### Schritt 1 — README

@call patch("README.md", "the CLI code block (lines 19-23) and the `check` bullet (line 28)")

- Im Code-Block nach `lean-md check  <file.lmd.md>`:
  `lean-md outline <file.lmd.md|-> --json [--require-phase a,b]`
- Nach dem `check`-Stichpunkt:

      - `outline` prints a document's structure as one JSON object without rendering:
        phases with their `@call`s, the macro signatures in scope, and findings (unknown
        macro, wrong argument count, malformed `@call`, duplicate phase, unresolvable
        `@import`, missing required phase). Exit 0 without findings, 1 with findings,
        2 on unusable input.

### Schritt 2 — CHANGELOG und Versionen

@call patch("CHANGELOG.md", "the line before `## [binary 0.2.3] — 2026-08-31` (line 11)")

Vor `## [binary 0.2.3] — 2026-08-31` einfügen:

    ## [binary 0.2.4] — 2026-09-14

    ### Added
    - `lean-md outline <file|-> --json [--require-phase a,b]`: the structure of an `.lmd.md`
      document as one JSON object — phases in document order with their `@call`s (arguments
      split exactly as a render splits them), the macro signatures in scope, and every
      finding a render would only hit later: unknown macro, wrong argument count, malformed
      `@call`, duplicate phase, unresolvable `@import`, missing required phase. No render,
      no bridge, no session sink, no file written. Exit 0 without findings, 1 with
      findings, 2 on unusable input.

Versionen, jeweils `0.2.3` → `0.2.4`:

- `Cargo.toml:3` — `version = "0.2.4"`
- `Cargo.lock:440` — der `version`-Eintrag unter `name = "lean-md"`
- `lean-ctx-addon.toml:4` — `version = "0.2.4"` im Block `[addon]`; `[artifacts.*]` bleibt unverändert.

### Schritt 3 — Gesamtlauf

Run: {{ var test_cmd }}
Expected: PASS, insbesondere `determinism`, `seed_history`, `list_phases`, `check_exit`, `outline`.

Run: {{ var lint_cmd }}
Expected: keine Warnung.

### Schritt 4 — Abnahme am lean-herdr-Plan

Run (Arbeitsverzeichnis `/home/tholo/Scripts/lean-herdr`):
`cargo run -q --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml --bin lean-md -- outline docs/lean-md/plans/2026-09-14-lean-herdr-rollen-und-routing.lmd.md --json`

Expected: JSON auf stdout mit den Phasen `task-1` bis `task-9` in dieser Reihenfolge. Jeder
Eintrag in `errors` wird im Bericht mit Zeile genannt und von Hand als echter Fehler im Plan
bestätigt; ein Befund, der kein echter Fehler ist, beendet die Task mit BLOCKED.

@call verify(README.md CHANGELOG.md Cargo.toml Cargo.lock lean-ctx-addon.toml)
@call commit("README.md CHANGELOG.md Cargo.toml Cargo.lock lean-ctx-addon.toml", "chore(release): prepare binary 0.2.4")
@call remember_decision("lean-md 0.2.4 prepared: `lean-md outline <file|-> --json [--require-phase a,b]` prints phases, @calls, macro signatures and findings (unknown_macro, arity, malformed_call, duplicate_phase, import, missing_phase) as JSON; exit 0/1/2. Tag v0.2.4 and the local install from the GitHub asset are the maintainer's steps.")
@phase-end
