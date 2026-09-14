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
}
