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
use crate::phases::{PhaseBlock, fenced_mask, parse_phase_name, phase_blocks, phase_title};

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

/// `Some(Ok(call))` for a well-formed `@call` line, `Some(Err(()))` for a malformed one,
/// `None` for any other line — an indented one included: a render reads a directive only
/// when its `@` starts the line, so `  @call x() /` renders as text.
fn call_at(line: usize, text: &str) -> Option<Result<OutlineCall, ()>> {
    let (name, args) = parse_directive_line(text.as_bytes())?;
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

/// Whether a `@define` line's `rest` (the text right after the `@define` prefix, not yet
/// trimmed) opens a macro body — mirrors `extract_definitions`'s Pass-1 rule (spec §2.3):
/// an empty header, one starting with `-end`, or one that fails `parse_call_signature`
/// never opens a body, so the line is dropped without swallowing what follows.
fn opens_define_body(rest: &str) -> bool {
    let header = rest.trim();
    if header.is_empty() || header.starts_with("-end") {
        return false;
    }
    parse_call_signature(header).is_some()
}

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
        // Pass-1 lines (`@define`/`@define-end`/`@import`) are read exactly like
        // `extract_definitions`: fence-blind. Only the `@call` check below is fenced.
        if in_define {
            if trimmed.starts_with("@define-end") {
                in_define = false;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("@import") {
            let target = rest.trim().trim_end_matches('/').trim();
            if !target.is_empty() {
                import_errors(ctx, target, line, &mut seen_imports, &mut out);
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("@define") {
            if opens_define_body(rest) {
                in_define = true;
            }
            continue;
        }
        if fenced[idx] {
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
                // Same Pass-1 order as `findings`, mirroring `extract_definitions`.
                if in_define {
                    if trimmed.starts_with("@define-end") {
                        in_define = false;
                    }
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("@import") {
                    let nested = rest.trim().trim_end_matches('/').trim();
                    if !nested.is_empty() {
                        import_errors(ctx, nested, line, seen, out);
                    }
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("@define") {
                    if opens_define_body(rest) {
                        in_define = true;
                    }
                    continue;
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
        assert_eq!(
            (p.name.as_str(), p.title.as_str(), p.line),
            ("task-1", "Task 1: first", 8)
        );
        assert_eq!(
            p.calls,
            vec![OutlineCall {
                macro_name: "route".to_string(),
                args: vec![
                    "implement".to_string(),
                    "core".to_string(),
                    "a.py b.py".to_string()
                ],
                line: 9,
            }]
        );
        assert_eq!(
            o.macros.get("route"),
            Some(&vec![
                "work".to_string(),
                "lane".to_string(),
                "files".to_string()
            ])
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
    fn any_indented_call_is_text_like_in_a_render() {
        let o = run("@phase \"a\"\n  @call nope() /\n\t@call nope() /\n @call g() /\n@phase-end\n");
        assert!(o.phases[0].calls.is_empty(), "{:?}", o.phases[0].calls);
        assert!(o.errors.is_empty(), "{:?}", o.errors);
    }

    #[test]
    fn quoted_commas_stay_in_one_argument() {
        let o = run("@phase \"a\"\n@call g(\"x, y\", z) /\n@phase-end\n");
        assert_eq!(
            o.phases[0].calls[0].args,
            vec!["x, y".to_string(), "z".to_string()]
        );
    }

    #[test]
    fn a_later_define_wins_in_macros() {
        let o = run("@define g(a)\nx\n@define-end\n@define g(a, b)\ny\n@define-end\n");
        assert_eq!(
            o.macros.get("g"),
            Some(&vec!["a".to_string(), "b".to_string()])
        );
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
        assert!(
            calls.borrow().is_empty(),
            "outline must not reach the backend: {:?}",
            calls.borrow()
        );
    }

    #[test]
    fn json_carries_the_three_top_level_keys_and_is_byte_stable() {
        let v = run(PLAN).to_json();
        assert_eq!(v["phases"][0]["calls"][0]["macro"], "route");
        assert_eq!(
            v["macros"]["route"],
            serde_json::json!(["work", "lane", "files"])
        );
        assert_eq!(v["errors"], serde_json::json!([]));
        assert_eq!(
            run(PLAN).to_json().to_string(),
            run(PLAN).to_json().to_string()
        );
    }

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
        assert_eq!(
            o.errors,
            vec![err(
                "unknown_macro",
                2,
                Some("task-1"),
                "macro not found: nope"
            )]
        );
    }

    #[test]
    fn arity_mismatch_is_reported() {
        let o = run("@define g(a, b)\nx\n@define-end\n@call g(1) /\n");
        assert_eq!(
            o.errors,
            vec![err("arity", 4, None, "g takes 2 argument(s), got 1")]
        );
    }

    #[test]
    fn malformed_call_is_reported() {
        let o = run("@call broken /\n");
        assert_eq!(
            o.errors,
            vec![err("malformed_call", 1, None, "malformed @call signature")]
        );
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
        assert!(
            o.errors[0]
                .message
                .starts_with("@import .lean-ctx/lean-md/nope failed"),
            "{:?}",
            o.errors
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_malformed_define_opens_no_body() {
        let o = run("@define foo\n@call nope() /\n@define-end\n");
        assert_eq!(
            o.errors,
            vec![err("unknown_macro", 2, None, "macro not found: nope")]
        );
        let o = run("@define\n@call nope() /\n");
        assert_eq!(
            o.errors,
            vec![err("unknown_macro", 2, None, "macro not found: nope")]
        );
    }

    #[test]
    fn a_fenced_import_is_checked_like_pass_one() {
        let dir =
            std::env::temp_dir().join(format!("lmd_outline_fenced_import_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let o = outline(
            "```\n@import .lean-ctx/lean-md/nope /\n```\n",
            dir.clone(),
            &[],
        );
        assert_eq!(o.errors.len(), 1, "{:?}", o.errors);
        assert_eq!((o.errors[0].kind, o.errors[0].line), ("import", 2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_fenced_define_body_still_hides_its_calls() {
        let o = run("```\n@define w()\n@call nope() /\n@define-end\n```\n@call nope2() /\n");
        assert_eq!(
            o.errors,
            vec![err("unknown_macro", 6, None, "macro not found: nope2")]
        );
    }

    #[test]
    fn a_missing_required_phase_is_reported() {
        let o = outline(
            "@phase \"a\"\n@phase-end\n",
            std::env::temp_dir(),
            &["a".to_string(), "lanes".to_string()],
        );
        assert_eq!(
            o.errors,
            vec![err(
                "missing_phase",
                0,
                Some("lanes"),
                "required @phase \"lanes\" is missing"
            )]
        );
    }

    #[test]
    fn calls_in_fences_and_define_bodies_are_not_checked() {
        let o = run("```\n@call nope() /\n```\n@define w()\n@call nope2() /\n@define-end\n");
        assert!(o.errors.is_empty(), "{:?}", o.errors);
    }

    #[test]
    fn errors_are_sorted_by_line_then_kind() {
        let o = outline(
            "@call b() /\n@call a(1) /\n",
            std::env::temp_dir(),
            &["x".to_string()],
        );
        let order: Vec<(usize, &str)> = o.errors.iter().map(|e| (e.line, e.kind)).collect();
        assert_eq!(
            order,
            vec![
                (0, "missing_phase"),
                (1, "unknown_macro"),
                (2, "unknown_macro")
            ]
        );
    }
}
