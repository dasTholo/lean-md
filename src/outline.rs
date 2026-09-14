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
use crate::phases::{fenced_mask, parse_phase_name, phase_title};

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
    // Only the body a render reads is outlined; `body` is a suffix of `source`, so the
    // header's newlines give the offset that keeps every line source-relative.
    let header_lines = source[..source.len() - body.len()].matches('\n').count();
    let scan = scan(body, header_lines);
    Outline {
        phases: scan.listed_phases(),
        macros,
        errors: findings(ctx, &scan, required),
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

/// The macro name when a `@define` line's `rest` (the text right after the `@define`
/// prefix, not yet trimmed) opens a body — `extract_definitions`'s Pass-1 rule (spec §2.3):
/// an empty header, one starting with `-end`, or one that fails `parse_call_signature`
/// opens none, so the line is dropped without swallowing what follows.
fn define_body_name(rest: &str) -> Option<String> {
    let header = rest.trim();
    if header.is_empty() || header.starts_with("-end") {
        return None;
    }
    parse_call_signature(header).map(|(name, _)| name)
}

/// The Pass-1 state machine of `extract_definitions`, shared by the document scan and the
/// import scan: fence-blind, an `@import` line read before a `@define` line.
#[derive(Default)]
struct Pass1 {
    /// `(line, name)` of the `@define` whose body is still open.
    open_define: Option<(usize, String)>,
}

/// What Pass 1 does with one line.
enum Pass1Line<'a> {
    /// Swallowed: a `@define` line, a line of an open body or its `@define-end`.
    Define,
    /// Swallowed: an `@import` line with its target (empty when it names none).
    Import(&'a str),
    /// Left to the later passes.
    Other,
}

impl Pass1 {
    fn step<'a>(&mut self, line: usize, text: &'a str) -> Pass1Line<'a> {
        let trimmed = text.trim_start();
        if self.open_define.is_some() {
            if trimmed.starts_with("@define-end") {
                self.open_define = None;
            }
            return Pass1Line::Define;
        }
        if let Some(rest) = trimmed.strip_prefix("@import") {
            return Pass1Line::Import(rest.trim().trim_end_matches('/').trim());
        }
        if let Some(rest) = trimmed.strip_prefix("@define") {
            self.open_define = define_body_name(rest).map(|name| (line, name));
            return Pass1Line::Define;
        }
        Pass1Line::Other
    }
}

/// How a render reads one scanned line, decided once by [`scan`].
enum LineKind<'a> {
    /// Swallowed by Pass 1: a `@define` line or a line of its body.
    Define,
    /// Swallowed by Pass 1: an `@import` line with its target (empty when it names none).
    Import(&'a str),
    /// An unfenced `@phase` line with its name.
    PhaseOpen(String),
    /// An unfenced `@phase-end` line.
    PhaseEnd,
    /// An active `@call` line; `Err` when its signature is malformed.
    Call(Result<OutlineCall, ()>),
    /// Anything else, fenced lines included.
    Text,
}

struct ScannedLine<'a> {
    /// 1-based line in the outlined source.
    line: usize,
    text: &'a str,
    kind: LineKind<'a>,
    /// Index into [`Scan::phases`] of the phase open at this line, its `@phase-end` included.
    phase: Option<usize>,
}

/// A top-level `@phase` line; `closed` once its `@phase-end` is met.
struct PhaseSite {
    name: String,
    line: usize,
    closed: bool,
}

struct Scan<'a> {
    lines: Vec<ScannedLine<'a>>,
    phases: Vec<PhaseSite>,
}

/// Every line of `text` classified once, in the order a render reads it: Pass 1
/// (fence-blind), then fences, then the flat `@phase` structure (a nested `@phase` leaves
/// the open phase open, like the render and `phase_blocks`) and `@call`s. `offset` is the
/// number of source lines before `text`.
fn scan(text: &str, offset: usize) -> Scan<'_> {
    let fenced = fenced_mask(text);
    let mut pass1 = Pass1::default();
    let mut phases: Vec<PhaseSite> = Vec::new();
    let mut open: Option<usize> = None;
    let mut lines = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = offset + idx + 1;
        let kind = classify(&mut pass1, fenced[idx], line, raw);
        let phase = open;
        match (&kind, open) {
            (LineKind::PhaseOpen(name), None) => {
                phases.push(PhaseSite {
                    name: name.clone(),
                    line,
                    closed: false,
                });
                open = Some(phases.len() - 1);
            }
            (LineKind::PhaseEnd, Some(i)) => {
                phases[i].closed = true;
                open = None;
            }
            _ => {}
        }
        lines.push(ScannedLine {
            line,
            text: raw,
            kind,
            phase,
        });
    }
    Scan { lines, phases }
}

/// One line's [`LineKind`]: Pass-1 lines first, fence-blind like `extract_definitions`;
/// every later rule only for an unfenced line.
fn classify<'a>(pass1: &mut Pass1, fenced: bool, line: usize, text: &'a str) -> LineKind<'a> {
    match pass1.step(line, text) {
        Pass1Line::Define => return LineKind::Define,
        Pass1Line::Import(target) => return LineKind::Import(target),
        Pass1Line::Other if fenced => return LineKind::Text,
        Pass1Line::Other => {}
    }
    let trimmed = text.trim_start();
    if trimmed.starts_with("@phase-end") {
        LineKind::PhaseEnd
    } else if let Some(rest) = trimmed.strip_prefix("@phase") {
        LineKind::PhaseOpen(parse_phase_name(rest))
    } else if let Some(call) = call_at(line, text) {
        LineKind::Call(call)
    } else {
        LineKind::Text
    }
}

impl Scan<'_> {
    fn phase_name(&self, phase: Option<usize>) -> Option<String> {
        phase.map(|i| self.phases[i].name.clone())
    }

    /// Every closed phase in document order with its active `@call`s — an unterminated
    /// block is not addressable, so it is not listed (same rule as `phase_blocks`).
    fn listed_phases(&self) -> Vec<OutlinePhase> {
        self.phases
            .iter()
            .enumerate()
            .filter(|(_, site)| site.closed)
            .map(|(i, site)| {
                let body: Vec<&ScannedLine> = self
                    .lines
                    .iter()
                    .filter(|l| l.phase == Some(i))
                    .filter(|l| !matches!(l.kind, LineKind::PhaseOpen(_) | LineKind::PhaseEnd))
                    .collect();
                let text: Vec<&str> = body.iter().map(|l| l.text).collect();
                let calls = body
                    .iter()
                    .filter_map(|l| match &l.kind {
                        LineKind::Call(Ok(call)) => Some(call.clone()),
                        _ => None,
                    })
                    .collect();
                OutlinePhase {
                    name: site.name.clone(),
                    title: phase_title(&text.join("\n")),
                    line: site.line,
                    calls,
                }
            })
            .collect()
    }
}

/// Every finding in `scan`, sorted by `(line, kind)`.
fn findings(ctx: &Rc<EngineContext>, scan: &Scan, required: &[String]) -> Vec<OutlineError> {
    let mut out = Vec::new();
    call_findings(ctx, scan, &mut out);
    phase_findings(scan, &mut out);
    import_findings(ctx, scan, &mut out);
    missing_phases(scan, required, &mut out);
    out.sort_by(|a, b| (a.line, a.kind).cmp(&(b.line, b.kind)));
    out
}

/// `malformed_call`, `unknown_macro` and `arity` for every active `@call` line.
fn call_findings(ctx: &Rc<EngineContext>, scan: &Scan, out: &mut Vec<OutlineError>) {
    for line in &scan.lines {
        let finding = match &line.kind {
            LineKind::Call(Err(())) => {
                Some(("malformed_call", "malformed @call signature".to_string()))
            }
            LineKind::Call(Ok(call)) => macro_mismatch(ctx, call),
            _ => None,
        };
        if let Some((kind, message)) = finding {
            out.push(OutlineError {
                kind,
                line: line.line,
                phase: scan.phase_name(line.phase),
                message,
            });
        }
    }
}

/// `unknown_macro` or `arity` for a well-formed `@call`; `None` when it fits its macro.
fn macro_mismatch(ctx: &Rc<EngineContext>, call: &OutlineCall) -> Option<(&'static str, String)> {
    let macros = ctx.macros.borrow();
    let Some(def) = macros.get(&call.macro_name) else {
        return Some((
            "unknown_macro",
            format!("macro not found: {}", call.macro_name),
        ));
    };
    (def.params.len() != call.args.len()).then(|| {
        let message = format!(
            "{} takes {} argument(s), got {}",
            call.macro_name,
            def.params.len(),
            call.args.len()
        );
        ("arity", message)
    })
}

/// `duplicate_phase` — the rule `phases::duplicate_phase` applies: every `@phase` line a
/// render reads counts, a nested one too.
fn phase_findings(scan: &Scan, out: &mut Vec<OutlineError>) {
    let mut first_seen: Vec<(&str, usize)> = Vec::new();
    for line in &scan.lines {
        let LineKind::PhaseOpen(name) = &line.kind else {
            continue;
        };
        match first_seen.iter().find(|(seen, _)| *seen == name.as_str()) {
            Some((_, first)) => out.push(OutlineError {
                kind: "duplicate_phase",
                line: line.line,
                phase: Some(name.clone()),
                message: format!(
                    "duplicate @phase \"{name}\" — first defined at line {first}, again at line {}",
                    line.line
                ),
            }),
            None => first_seen.push((name, line.line)),
        }
    }
}

/// `import` for every `@import` whose target, or a library it imports, fails to resolve.
fn import_findings(ctx: &Rc<EngineContext>, scan: &Scan, out: &mut Vec<OutlineError>) {
    let mut seen = Vec::new();
    for line in &scan.lines {
        if let LineKind::Import(target) = line.kind
            && !target.is_empty()
        {
            import_errors(ctx, target, line.line, &mut seen, out);
        }
    }
}

/// `missing_phase` for every required name no listed phase carries.
fn missing_phases(scan: &Scan, required: &[String], out: &mut Vec<OutlineError>) {
    for name in required {
        if !scan
            .phases
            .iter()
            .any(|site| site.closed && site.name == *name)
        {
            out.push(OutlineError {
                kind: "missing_phase",
                line: 0,
                phase: Some(name.clone()),
                message: format!("required @phase \"{name}\" is missing"),
            });
        }
    }
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
            let mut pass1 = Pass1::default();
            for (idx, text) in content.lines().enumerate() {
                if let Pass1Line::Import(nested) = pass1.step(idx + 1, text)
                    && !nested.is_empty()
                {
                    import_errors(ctx, nested, line, seen, out);
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

    #[test]
    fn a_call_inside_a_define_body_in_a_phase_is_not_listed() {
        let o = run(
            "@define g()\nx\n@define-end\n@phase \"t\"\n@define w()\n@call g() /\n@define-end\n@call g() /\n@phase-end\n",
        );
        let lines: Vec<usize> = o.phases[0].calls.iter().map(|c| c.line).collect();
        assert_eq!(lines, vec![8]);
        assert!(o.errors.is_empty(), "{:?}", o.errors);
    }

    #[test]
    fn a_header_without_a_blank_line_leaves_no_body_to_outline() {
        let o = run("@lean-md\nconsumer: ai\n@phase \"t\"\n@call g() /\n@phase-end\n");
        assert!(o.phases.is_empty(), "{:?}", o.phases);
        assert!(o.errors.is_empty(), "{:?}", o.errors);
    }

    #[test]
    fn body_findings_keep_their_source_line_after_a_header() {
        let o = run("@lean-md\nconsumer: ai\n\n@phase \"t\"\n@call nope() /\n@phase-end\n");
        assert_eq!(o.phases[0].line, 4);
        assert_eq!(
            o.errors,
            vec![err("unknown_macro", 5, Some("t"), "macro not found: nope")]
        );
    }
}
