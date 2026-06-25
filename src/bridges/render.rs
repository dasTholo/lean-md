//! `@render type=table|list` bridge (spec §5, E-#5). A pure formatter: it
//! REQUIRES `piped_input` (standalone `@render` is meaningless → MissingArg).
//! Deterministic + headless/golden-testable. `table` renders line/column-
//! oriented input as a Markdown table (best-effort, whitespace-split columns);
//! `list` renders each non-empty line as a `- ` bullet. Custom types
//! (mermaid/redact) are a Phase-5 extension (spec §9) — not built-in here.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct RenderBridge;

impl DirectiveBridge for RenderBridge {
    fn name(&self) -> &'static str {
        "render"
    }
    fn accepts_pipe(&self) -> bool {
        true
    }
    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let input = args
            .piped_input()
            .ok_or(BridgeError::MissingArg("piped_input"))?;
        let kind = args
            .get("type")
            .or_else(|| args.positional(0))
            .unwrap_or("list");
        match kind {
            "list" => Ok(render_list(input)),
            "table" => Ok(render_table(input)),
            other => {
                // Phase 5: before the error, try a registered RenderTransform.
                // WASM transforms are self-contained (empty-linker sandbox) — no
                // header gate. `hint` carries the @consumer audience (ai 0/human 1).
                let hint = ctx.consumer_hint();
                if let Some(t) = crate::core::extension_registry::global()
                    .read()
                    .ok()
                    .and_then(|r| r.render_transform(other))
                {
                    return Ok(t.render(input, hint));
                }
                Err(BridgeError::Resolve(format!(
                    "unknown @render type '{other}'. Use: table|list (or a \
                     registered render transform; custom types require the \
                     `wasm` feature)"
                )))
            }
        }
    }
}

fn render_list(input: &str) -> String {
    let mut out = String::new();
    for line in input.lines().filter(|l| !l.trim().is_empty()) {
        out.push_str("- ");
        out.push_str(line.trim());
        out.push('\n');
    }
    out
}

fn render_table(input: &str) -> String {
    let rows: Vec<Vec<&str>> = input
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split_whitespace().collect())
        .collect();
    if rows.is_empty() {
        return String::new();
    }
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    out.push_str("| ");
    out.push_str(&pad_row(&rows[0], cols).join(" | "));
    out.push_str(" |\n| ");
    out.push_str(&vec!["---"; cols].join(" | "));
    out.push_str(" |\n");
    for row in &rows[1..] {
        out.push_str("| ");
        out.push_str(&pad_row(row, cols).join(" | "));
        out.push_str(" |\n");
    }
    out
}

fn pad_row<'a>(row: &[&'a str], cols: usize) -> Vec<&'a str> {
    let mut r = row.to_vec();
    r.resize(cols, "");
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn render_is_registered_and_accepts_pipe() {
        let reg = super::super::default_registry();
        let b = reg.get("render").expect("render registered");
        assert!(b.accepts_pipe(), "@render must accept pipe");
    }

    #[test]
    fn render_without_pipe_errors() {
        let err = RenderBridge
            .execute(&ctx(), &DirectiveArgs::parse("type=list"))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("piped_input")));
    }

    #[test]
    fn render_list_bullets_each_line() {
        let args = DirectiveArgs::parse("type=list").with_piped_input("a\nb\n\nc".into());
        let out = RenderBridge.execute(&ctx(), &args).unwrap();
        assert_eq!(out, "- a\n- b\n- c\n");
    }

    #[test]
    fn render_table_builds_markdown_table() {
        let args = DirectiveArgs::parse("type=table")
            .with_piped_input("name age\nalice 30\nbob 25".into());
        let out = RenderBridge.execute(&ctx(), &args).unwrap();
        assert!(out.contains("| name | age |"), "got: {out}");
        assert!(out.contains("| --- | --- |"), "got: {out}");
        assert!(out.contains("| alice | 30 |"), "got: {out}");
    }
    #[test]
    fn unknown_type_degrades_to_clear_message_not_panic() {
        let args = DirectiveArgs::parse("type=mermaid").with_piped_input("a\nb".into());
        let err = RenderBridge.execute(&ctx(), &args).unwrap_err();
        match err {
            BridgeError::Resolve(m) => {
                assert!(m.contains("unknown @render type 'mermaid'"), "got: {m}");
                assert!(
                    m.contains("registered render transform"),
                    "must mention the extension path: {m}"
                );
            }
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn registered_render_transform_resolves_before_error() {
        use crate::core::extension_registry::{RenderTransform, global};
        use std::sync::Arc;

        struct Shout;
        #[allow(clippy::unnecessary_literal_bound)]
        impl RenderTransform for Shout {
            fn name(&self) -> &str {
                "shout_test"
            }
            fn render(&self, input: &str, hint: i32) -> String {
                format!("{}!{hint}", input.trim().to_uppercase())
            }
        }
        global()
            .write()
            .unwrap()
            .register_render_transform(Arc::new(Shout));

        let args = DirectiveArgs::parse("type=shout_test").with_piped_input("hi".into());
        // Default ctx() => consumer ai => hint 0.
        let out = RenderBridge.execute(&ctx(), &args).unwrap();
        assert_eq!(out, "HI!0");
    }
}
