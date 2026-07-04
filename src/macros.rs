//! lmd Macro-Engine + Container pre-passes (spec §2.2/§2.3/§4). These run as
//! line-based text→text transforms INSIDE `render_body`, BEFORE rushdown sees
//! the body — exactly the `parse_header` pre-scan pattern (spec §2.2.1). The
//! definition space (`@define`/`@import`) is stripped here and emits nothing;
//! the output space (`@call`, container survivors) flows on to rushdown.
//! Phase 4A: types, signature/param helpers, eval_condition, extract_definitions.
//! Phase 4B adds prune_containers + eval_string.

use std::collections::HashMap;
use std::rc::Rc;

use evalexpr::{
    ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value,
    eval_boolean_with_context, eval_with_context,
};

use super::engine::EngineContext;
use super::header::Consumer;

/// One authored macro: `@define name(p1, p2) … @define-end`.
#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: String,
}

/// Built-in-first macro registry (Phase 4 ships no built-in macros yet — the
/// `orient` built-in is Phase 6, spec §9 — so this holds authored defs only).
#[derive(Debug, Default)]
pub struct MacroRegistry {
    authored: HashMap<String, MacroDef>,
}

impl MacroRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    /// Insert (or overwrite) an authored macro. Last `@define` wins — a later
    /// definition shadows an earlier same-name one (spec §7 duplicate rule;
    /// built-in-first shadow-warn is Phase 6 when built-ins exist).
    pub fn insert_authored(&mut self, def: MacroDef) {
        self.authored.insert(def.name.clone(), def);
    }
    pub fn get(&self, name: &str) -> Option<&MacroDef> {
        self.authored.get(name)
    }
    pub fn len(&self) -> usize {
        self.authored.len()
    }
    pub fn is_empty(&self) -> bool {
        self.authored.is_empty()
    }
    /// Project each authored macro to a one-line signature `name(p1, p2) — <doc>`,
    /// where <doc> is the first non-empty body line with HTML-comment markers
    /// stripped (the description convention). Names are sorted so the output is a
    /// deterministic function of the registry (#498). Bodies are NOT emitted.
    pub fn signature_index(&self) -> String {
        let mut names: Vec<&String> = self.authored.keys().collect();
        names.sort();
        let mut out = String::new();
        for name in names {
            let def = self.authored.get(name).expect("key from same map");
            let raw = def
                .body
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .trim();
            let doc = raw
                .trim_start_matches("<!--")
                .trim_end_matches("-->")
                .trim();
            out.push_str(&format!(
                "{}({}) — {}\n",
                def.name,
                def.params.join(", "),
                doc
            ));
        }
        out
    }
}

/// Parse a `@call` signature `name(arg1, arg2) /` → (`name`, [`arg1`,`arg2`]).
/// The trailing ` /` self-close is optional. Each comma-separated arg is
/// trimmed (paths/spaces survive per-segment, spec §3.1). Returns None if the
/// parenthesized form is malformed (no `(`/`)`), so the caller can error cleanly.
pub fn parse_call_signature(raw: &str) -> Option<(String, Vec<String>)> {
    let raw = raw.trim().trim_end_matches('/').trim();
    let open = raw.find('(')?;
    let close = raw.rfind(')')?;
    if close < open {
        return None;
    }
    let name = raw[..open].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let inner = &raw[open + 1..close];
    let args: Vec<String> = if inner.trim().is_empty() {
        Vec::new()
    } else {
        split_call_args(inner)
    };
    Some((name, args))
}

/// Strip exactly ONE surrounding pair of double quotes (if present) from an
/// already-trimmed arg segment. Leaves nested/inner quotes intact so
/// `""a""` → `"a"`, not `a`.
fn strip_one_quote_layer(s: &str) -> String {
    let s = s.trim();
    s.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(s)
        .to_string()
}

/// Split call args on top-level commas — commas inside `"..."` are literal —
/// then strip one surrounding layer of double quotes from each segment.
/// Unquoted segments keep their legit top-level-comma separation (multi-arg
/// recipes like `reformat_commit(paths, msg)`), so this never breaks the
/// existing unquoted call form.
fn split_call_args(inner: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in inner.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(ch);
            }
            ',' if !in_quotes => {
                args.push(strip_one_quote_layer(&cur));
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    args.push(strip_one_quote_layer(&cur));
    args
}

/// Substitute `{{ p }}` (flexible inner whitespace) for each `params[i]` with
/// the matching `args[i]` in `body`. Textual — runs at `@call` time BEFORE
/// `render_body`, so the substituted body's directives then dispatch normally
/// (spec §4 "textual {{ p }} interpolation in body content"). Missing args
/// substitute the empty string (passive-macro tolerance).
pub fn substitute_params(body: &str, params: &[String], args: &[String]) -> String {
    let mut out = body.to_string();
    for (i, p) in params.iter().enumerate() {
        let val = args.get(i).map_or("", String::as_str);
        for needle in [
            format!("{{{{ {p} }}}}"),
            format!("{{{{{p}}}}}"),
            format!("{{{{  {p}  }}}}"),
        ] {
            out = out.replace(&needle, val);
        }
    }
    out
}

/// Evaluate an `@if`/`@elseif` boolean condition over the in-memory variable
/// context: `consumer`/`version`/`shell` (header), bound macro-params (current
/// `param_scope` top), and every `env.NAME` token referenced in the expression
/// (resolved from the process env). Pure — no I/O beyond env reads (spec §4).
pub fn eval_condition(ctx: &Rc<EngineContext>, expr: &str) -> Result<bool, String> {
    let mut context = HashMapContext::<DefaultNumericTypes>::new();
    let header = &ctx.header;

    let consumer = match header.consumer {
        Consumer::Ai => "ai",
        Consumer::Human => "human",
    };
    let _ = context.set_value("consumer".into(), Value::from(consumer));
    let _ = context.set_value(
        "version".into(),
        Value::from(header.version.clone().unwrap_or_default()),
    );
    let shell = match header.shell {
        super::header::ShellMode::Allow => "allow",
        super::header::ShellMode::Deny => "deny",
    };
    let _ = context.set_value("shell".into(), Value::from(shell));

    for (k, v) in ctx.param_scope.borrow().last().cloned().unwrap_or_default() {
        let _ = context.set_value(k, Value::from(v));
    }

    for name in scan_env_refs(expr) {
        let val = std::env::var(&name[4..]).unwrap_or_default(); // strip "env."
        let _ = context.set_value(name, Value::from(val));
    }

    eval_boolean_with_context(expr, &context).map_err(|e| e.to_string())
}

/// Collect distinct `env.NAME` tokens (NAME = `[A-Za-z_][A-Za-z0-9_]*`).
pub(super) fn scan_env_refs(expr: &str) -> Vec<String> {
    let bytes = expr.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(pos) = expr[i..].find("env.") {
        let start = i + pos;
        let mut j = start + 4;
        while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            j += 1;
        }
        if j > start + 4 {
            let tok = expr[start..j].to_string();
            if !out.contains(&tok) {
                out.push(tok);
            }
        }
        i = j.max(start + 4);
    }
    out
}

/// Evaluate an inline `{{ expr }}` to a DISPLAY string (spec §3.1 third tier).
/// Tries boolean first (so `{{ a == "b" }}` → "true"/"false"), then a general
/// evalexpr eval (numbers/strings). On error returns an inline error comment so
/// the render stays visible (never aborts).
pub fn eval_string(ctx: &Rc<EngineContext>, expr: &str) -> String {
    if let Ok(b) = eval_condition(ctx, expr) {
        return b.to_string();
    }
    match eval_value(ctx, expr) {
        Ok(v) => value_to_string(&v),
        Err(e) => format!("<!-- lmd:{{{{ }}}} eval err: {e} -->"),
    }
}

fn eval_value(ctx: &Rc<EngineContext>, expr: &str) -> Result<Value, String> {
    let mut context = HashMapContext::new();
    let header = &ctx.header;
    let consumer = match header.consumer {
        Consumer::Ai => "ai",
        Consumer::Human => "human",
    };
    let _ = context.set_value("consumer".into(), Value::from(consumer));
    let _ = context.set_value(
        "version".into(),
        Value::from(header.version.clone().unwrap_or_default()),
    );
    for (k, v) in ctx.param_scope.borrow().last().cloned().unwrap_or_default() {
        let _ = context.set_value(k, Value::from(v));
    }
    for name in scan_env_refs(expr) {
        let val = std::env::var(&name[4..]).unwrap_or_default();
        let _ = context.set_value(name, Value::from(val));
    }
    eval_with_context(expr, &context).map_err(|e| e.to_string())
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        other => format!("{other:?}"),
    }
}

/// Pass 1 (spec §2.3): line-scan `input`, pulling `@define name(params) …
/// @define-end` blocks and `@import target /` lines into `ctx.macros` and
/// STRIPPING them from the returned text (the definition space is invisible).
/// Forward references are free: every define is registered before any `@call`
/// renders. Re-entrant-safe: re-running on a macro/fragment body is a cheap
/// no-op when it carries no definition lines; `@import` is deduped per render.
pub fn extract_definitions(ctx: &Rc<EngineContext>, input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut lines = input.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("@import") {
            let target = rest.trim().trim_end_matches('/').trim();
            if !target.is_empty() {
                import_library(ctx, target, &mut out);
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@define") {
            let header = rest.trim();
            if header.is_empty() || header.starts_with("-end") {
                continue; // stray `@define-end` — drop silently
            }
            match parse_call_signature(header) {
                Some((name, params)) => {
                    let mut body = String::new();
                    let mut closed = false;
                    for inner in lines.by_ref() {
                        if inner.trim_start().starts_with("@define-end") {
                            closed = true;
                            break;
                        }
                        body.push_str(inner);
                        body.push('\n');
                    }
                    if !closed {
                        out.push_str(&format!("<!-- lmd: unterminated @define {name} -->\n"));
                        break;
                    }
                    ctx.macros
                        .borrow_mut()
                        .insert_authored(MacroDef { name, params, body });
                }
                None => {
                    out.push_str("<!-- lmd: malformed @define signature -->\n");
                }
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Pass 3 (spec §2.3/§4): line-scan `input`, replacing each `@if … @if-end` /
/// `@consumer … @consumer-end` container with the body of its first matching
/// branch (raw — re-rendered downstream by rushdown). Branches evaluate
/// top-to-bottom; first `cond == true` wins; else the `@else` body; else empty.
/// `@consumer X` is sugar for `@if consumer == "X"`. An eval error skips the
/// container (no branch) and emits a visible comment; an unterminated block
/// emits a comment and stops (spec §7 — render never aborts).
pub fn prune_containers(ctx: &Rc<EngineContext>, input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut lines = input.lines();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        let opener = if let Some(rest) = trimmed.strip_prefix("@if") {
            // Skip `@if-end`/`@ifdef`-ish strays at top level (no open container).
            if rest.starts_with("-end") {
                None
            } else {
                Some(rest.trim().to_string())
            }
        } else if let Some(rest) = trimmed.strip_prefix("@consumer") {
            if rest.starts_with("-end") {
                None
            } else {
                Some(format!("consumer == \"{}\"", rest.trim()))
            }
        } else {
            None
        };

        let Some(first_cond) = opener else {
            out.push_str(line);
            out.push('\n');
            continue;
        };

        let is_consumer = trimmed.starts_with("@consumer");
        let end_marker = if is_consumer {
            "@consumer-end"
        } else {
            "@if-end"
        };

        // Collect branches: (Option<cond_expr>, body). `@else` → None cond.
        let mut branches: Vec<(Option<String>, String)> = vec![(Some(first_cond), String::new())];
        let mut closed = false;
        for inner in lines.by_ref() {
            let it = inner.trim_start();
            if it.starts_with(end_marker) {
                closed = true;
                break;
            }
            if !is_consumer && it.starts_with("@elseif") {
                let cond = it.trim_start_matches("@elseif").trim().to_string();
                branches.push((Some(cond), String::new()));
                continue;
            }
            if !is_consumer && it.starts_with("@else") {
                branches.push((None, String::new()));
                continue;
            }
            let last = branches.last_mut().unwrap();
            last.1.push_str(inner);
            last.1.push('\n');
        }

        if !closed {
            out.push_str("<!-- lmd: unterminated @if -->\n");
            continue;
        }

        for (cond, body) in &branches {
            if let Some(expr) = cond {
                match eval_condition(ctx, expr) {
                    Ok(true) => {
                        out.push_str(body);
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        out.push_str(&format!("<!-- lmd:@if eval err: {e} -->\n"));
                        break; // container handled (skipped)
                    }
                }
            } else {
                out.push_str(body); // @else — no prior cond matched
                break;
            }
        }
        // No-match-no-else → nothing emitted, by design.
    }
    out
}

/// Load `<target>.lmd.md` (built-in-first via the fragment registry, jailed),
/// recurse `extract_definitions` over it to register its macros, emit nothing.
/// Deduped: a library is loaded at most once per render.
fn import_library(ctx: &Rc<EngineContext>, target: &str, out: &mut String) {
    let candidate = ctx.jail_root.join(format!("{target}.lmd.md"));
    if !ctx.mark_imported(&candidate) {
        return;
    }
    match ctx.fragments.resolve(target, &ctx.jail_root) {
        Ok(content) => {
            let _ = extract_definitions(ctx, &content);
        }
        Err(e) => {
            out.push_str(&format!("<!-- lmd: @import {target} failed: {e:?} -->\n"));
        }
    }
}

/// Load every `@define` from `source` into a throwaway registry and return the
/// macro signature index (see `MacroRegistry::signature_index`). Used by
/// `lean-md render <lib> --signatures` so a plan author reads the macro API
/// instead of the whole library.
pub fn render_signature_index(source: &str, jail_root: std::path::PathBuf) -> String {
    use std::rc::Rc;
    let (header, body) = crate::header::parse_header(source);
    let ctx = Rc::new(crate::engine::EngineContext::new(header, jail_root));
    let _ = extract_definitions(&ctx, body);
    ctx.macros.borrow().signature_index()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineContext;
    use crate::header::{Consumer, LeanMdHeader};
    use std::path::PathBuf;

    fn ctx_with(header: LeanMdHeader) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(header, PathBuf::from(".")))
    }

    #[test]
    fn parses_call_signature_with_args() {
        let (n, a) = parse_call_signature("format(rust/src, --check) /").unwrap();
        assert_eq!(n, "format");
        assert_eq!(a, vec!["rust/src".to_string(), "--check".to_string()]);
    }

    #[test]
    fn parses_call_signature_no_args() {
        let (n, a) = parse_call_signature("orient()").unwrap();
        assert_eq!(n, "orient");
        assert!(a.is_empty());
    }

    #[test]
    fn rejects_malformed_signature() {
        assert!(parse_call_signature("no-parens").is_none());
    }

    #[test]
    fn substitutes_named_params() {
        let body = "files: {{ target }} mode {{mode}}";
        let out = substitute_params(
            body,
            &["target".into(), "mode".into()],
            &["rust/src".into(), "full".into()],
        );
        assert_eq!(out, "files: rust/src mode full");
    }

    #[test]
    fn eval_condition_reads_consumer() {
        let h = LeanMdHeader {
            consumer: Consumer::Human,
            ..Default::default()
        };
        let ctx = ctx_with(h);
        assert!(eval_condition(&ctx, r#"consumer == "human""#).unwrap());
        assert!(!eval_condition(&ctx, r#"consumer == "ai""#).unwrap());
    }

    #[test]
    fn eval_condition_reads_env() {
        crate::test_env::set_var("LMD_PHASE4_CI", "true");
        let ctx = ctx_with(LeanMdHeader::default());
        assert!(eval_condition(&ctx, r#"env.LMD_PHASE4_CI == "true""#).unwrap());
        crate::test_env::remove_var("LMD_PHASE4_CI");
    }

    #[test]
    fn eval_condition_reads_bound_param() {
        let ctx = ctx_with(LeanMdHeader::default());
        let mut scope = HashMap::new();
        scope.insert("lang".to_string(), "rust".to_string());
        ctx.push_params(scope);
        assert!(eval_condition(&ctx, r#"lang == "rust""#).unwrap());
        ctx.pop_params();
    }

    #[test]
    fn eval_condition_surfaces_error() {
        let ctx = ctx_with(LeanMdHeader::default());
        assert!(eval_condition(&ctx, "undefined_ident == 1").is_err());
    }

    #[test]
    fn extract_define_fills_registry_and_strips_body() {
        let ctx = ctx_with(LeanMdHeader::default());
        let input = "before\n@define greet(name)\nhi {{ name }}\n@define-end\nafter\n";
        let stripped = extract_definitions(&ctx, input);
        assert!(stripped.contains("before") && stripped.contains("after"));
        assert!(
            !stripped.contains("@define") && !stripped.contains("hi {{ name }}"),
            "definition space leaked into body: {stripped}"
        );
        let reg = ctx.macros.borrow();
        let def = reg.get("greet").expect("greet must be registered");
        assert_eq!(def.params, vec!["name".to_string()]);
        assert_eq!(def.body.trim(), "hi {{ name }}");
    }

    #[test]
    fn unterminated_define_emits_comment_not_panic() {
        let ctx = ctx_with(LeanMdHeader::default());
        let stripped = extract_definitions(&ctx, "@define x()\nbody never closed\n");
        assert!(
            stripped.contains("unterminated @define"),
            "must surface a visible error, got: {stripped}"
        );
    }

    #[test]
    fn import_loads_jailed_library_invisibly() {
        let dir = std::env::temp_dir().join("lmd_p4_import");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("lib.lmd.md"),
            "@define libmac()\nFROM_LIB\n@define-end\n",
        )
        .unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let stripped = extract_definitions(&ctx, "@import lib /\nbody\n");
        assert!(stripped.contains("body"));
        assert!(!stripped.contains("@import"), "import must be invisible");
        assert!(
            ctx.macros.borrow().get("libmac").is_some(),
            "imported macro must register"
        );
    }

    #[test]
    fn prune_keeps_matching_if_branch() {
        let ctx = ctx_with(LeanMdHeader {
            consumer: Consumer::Human,
            ..Default::default()
        });
        let input = "@if consumer == \"human\"\nHUMAN_TEXT\n@elseif consumer == \"ai\"\nAI_TEXT\n@else\nOTHER\n@if-end\n";
        let out = prune_containers(&ctx, input);
        assert!(out.contains("HUMAN_TEXT"), "got: {out}");
        assert!(
            !out.contains("AI_TEXT") && !out.contains("OTHER"),
            "got: {out}"
        );
        assert!(
            !out.contains("@if"),
            "container markers must be stripped: {out}"
        );
    }

    #[test]
    fn prune_falls_through_to_else() {
        let ctx = ctx_with(LeanMdHeader::default()); // consumer = ai (default)
        let input = "@if consumer == \"human\"\nH\n@else\nE\n@if-end\n";
        let out = prune_containers(&ctx, input);
        assert!(out.contains('E') && !out.contains('H'), "got: {out}");
    }

    #[test]
    fn prune_no_match_no_else_is_empty() {
        let ctx = ctx_with(LeanMdHeader::default());
        let input = "@if consumer == \"human\"\nH\n@if-end\n";
        let out = prune_containers(&ctx, input);
        assert!(!out.contains('H'), "no branch must render: {out}");
    }

    #[test]
    fn consumer_sugar_equals_if_consumer() {
        let ctx = ctx_with(LeanMdHeader {
            consumer: Consumer::Human,
            ..Default::default()
        });
        let out = prune_containers(&ctx, "@consumer human\nONLY_HUMAN\n@consumer-end\n");
        assert!(out.contains("ONLY_HUMAN"), "got: {out}");
        let ctx_ai = ctx_with(LeanMdHeader::default());
        let out_ai = prune_containers(&ctx_ai, "@consumer human\nONLY_HUMAN\n@consumer-end\n");
        assert!(
            !out_ai.contains("ONLY_HUMAN"),
            "ai must drop human block: {out_ai}"
        );
    }

    #[test]
    fn prune_eval_error_skips_container_and_continues() {
        let ctx = ctx_with(LeanMdHeader::default());
        let input = "@if undefined_bareword\nX\n@if-end\nAFTER\n";
        let out = prune_containers(&ctx, input);
        assert!(
            out.contains("AFTER"),
            "render must continue past a bad @if: {out}"
        );
        assert!(
            out.contains("@if eval err"),
            "must surface the eval error: {out}"
        );
        assert!(
            !out.contains("\nX\n"),
            "errored container body must not render: {out}"
        );
    }

    #[test]
    fn unterminated_if_emits_comment() {
        let ctx = ctx_with(LeanMdHeader::default());
        let out = prune_containers(&ctx, "@if consumer == \"ai\"\nbody\n");
        assert!(out.contains("unterminated @if"), "got: {out}");
    }

    #[test]
    fn eval_string_renders_bool_and_var() {
        let ctx = ctx_with(LeanMdHeader {
            version: Some("0.4".to_string()),
            ..Default::default()
        });
        assert_eq!(eval_string(&ctx, "version"), "0.4");
        assert_eq!(eval_string(&ctx, r#"consumer == "ai""#), "true");
    }

    #[test]
    fn macro_signatures_extract() {
        // Two defines; each first body line is an HTML-comment description.
        let src = "\
@define commit(msg)
<!-- Stage and commit with the given message -->
```bash
git commit -m \"{{ msg }}\"
```
@define-end
@define test(name)
<!-- Run one test by name -->
Run: `{{ test_cmd }} {{ name }}`
@define-end
";
        let jail = std::path::PathBuf::from(".");
        let idx = render_signature_index(src, jail);

        // Both headers present, alphabetically sorted (commit before test).
        let commit_at = idx.find("commit(msg)").expect("commit signature missing");
        let test_at = idx.find("test(name)").expect("test signature missing");
        assert!(
            commit_at < test_at,
            "signatures must be name-sorted (deterministic)"
        );

        // Description lines projected, comment markers stripped.
        assert!(idx.contains("commit(msg) — Stage and commit with the given message"));
        assert!(idx.contains("test(name) — Run one test by name"));

        // Bodies stripped: no expansion lines leak into the index.
        assert!(
            !idx.contains("git commit -m"),
            "macro body must not appear in index"
        );
        assert!(
            !idx.contains("Run: `"),
            "macro body must not appear in index"
        );
    }

    #[test]
    fn call_args_split_is_quote_aware() {
        // Comma inside "..." must NOT split the arg; surrounding quotes are stripped.
        let (n, a) = parse_call_signature(r#"commit("src/foo.rs", "feat: add foo, bar")"#).unwrap();
        assert_eq!(n, "commit");
        assert_eq!(
            a,
            vec!["src/foo.rs".to_string(), "feat: add foo, bar".to_string()]
        );
    }

    #[test]
    fn call_args_strip_surrounding_quotes_no_leak() {
        let (_, a) = parse_call_signature(r#"commit("a", "b")"#).unwrap();
        assert_eq!(
            a,
            vec!["a".to_string(), "b".to_string()],
            "quotes must be stripped, none leaked"
        );
    }

    #[test]
    fn call_args_unquoted_still_split_on_top_level_comma() {
        // Regression: unquoted multi-arg recipes keep their legit comma separator.
        let (_, a) = parse_call_signature("format(rust/src, --check)").unwrap();
        assert_eq!(a, vec!["rust/src".to_string(), "--check".to_string()]);
    }

    #[test]
    fn call_expands_quoted_comma_arg_without_quote_leak() {
        // End-to-end: a quoted comma arg must survive the full render pipeline.
        let out = crate::engine::render(
            "@define note(a, b)\n[{{ a }}|{{ b }}]\n@define-end\n\n@call note(\"x\", \"y, z\") /\n",
        );
        assert!(
            out.contains("[x|y, z]"),
            "quoted comma arg must survive intact: {out}"
        );
        assert!(!out.contains('"'), "no quote leak: {out}");
    }

    #[test]
    fn preflight_bug1_commit_quoted_comma_arg_survives() {
        // Spec Preflight gate: the exact repro the terseness template will rely on —
        // a quoted comma-bearing message must survive as ONE arg with its inner comma,
        // with no quote leak. Proves Bug 1 (30d872e) is present before later tasks
        // introduce quoted recipe args.
        let (name, args) =
            parse_call_signature(r#"commit("src/foo.rs", "feat: add foo, bar")"#).unwrap();
        assert_eq!(name, "commit");
        assert_eq!(
            args,
            vec!["src/foo.rs".to_string(), "feat: add foo, bar".to_string()],
            "quoted comma-bearing arg must survive intact, no quote leak"
        );
    }

    #[test]
    fn call_args_strip_only_one_quote_layer() {
        // Follow-up A: single-layer stripping — a nested quote survives.
        let (_, a) = parse_call_signature(r#"note(""a"", "b")"#).unwrap();
        assert_eq!(
            a,
            vec!["\"a\"".to_string(), "b".to_string()],
            "only one surrounding quote layer may be stripped"
        );
    }
}
