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
    eval_boolean_with_context,
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
        inner.split(',').map(|s| s.trim().to_string()).collect()
    };
    Some((name, args))
}

/// Substitute `{{ p }}` (flexible inner whitespace) for each `params[i]` with
/// the matching `args[i]` in `body`. Textual — runs at `@call` time BEFORE
/// `render_body`, so the substituted body's directives then dispatch normally
/// (spec §4 "textuelle {{ p }}-Interpolation im Body-Content"). Missing args
/// substitute the empty string (passive-macro tolerance).
pub fn substitute_params(body: &str, params: &[String], args: &[String]) -> String {
    let mut out = body.to_string();
    for (i, p) in params.iter().enumerate() {
        let val = args.get(i).map(String::as_str).unwrap_or("");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::engine::EngineContext;
    use crate::lmd::header::{Consumer, LeanMdHeader};
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
        let mut h = LeanMdHeader::default();
        h.consumer = Consumer::Human;
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
}
