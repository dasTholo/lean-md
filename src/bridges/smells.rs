//! `@smells` bridge → code-smell detection via `ctx_smells` (v1 §4.4).
//! Headless. Read-only. `action` (scan|summary|rules|file) is positional-0,
//! default `scan` — a DELIBERATE lmd default: the `ctx_smells` MCP wrapper
//! defaults to `summary`, but a directive should surface findings, not just
//! counts (cf. `@find` bm25-vs-hybrid). `rule=`/`path=` optional; `path` is an
//! FS filter and is jail-resolved. No `format=` exposed (backend default
//! `text` — "erben, nicht neu erfinden", §5).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct SmellsBridge;

impl DirectiveBridge for SmellsBridge {
    fn name(&self) -> &'static str {
        "smells"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let action = match args
            .positional(0)
            .or_else(|| args.get("action"))
            .unwrap_or("scan")
        {
            a @ ("scan" | "summary" | "rules" | "file") => a,
            other => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @smells action '{other}'. Use: scan|summary|rules|file"
                )));
            }
        };

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let rule = args.get("rule"); // optional rule-name filter

        // `path` is an optional FS filter; jail-resolve when present (design §4.4).
        let resolved_path: Option<String> = match args.get("path") {
            Some(p) => Some(
                crate::pathx::resolve_tool_path(Some(root), None, p)
                    .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?,
            ),
            None => None,
        };

        let mut payload = serde_json::Map::new();
        payload.insert("action".into(), action.into());
        if let Some(r) = rule {
            payload.insert("rule".into(), r.into());
        }
        if let Some(ref p) = resolved_path {
            payload.insert("path".into(), p.clone().into());
        }
        let out = ctx
            .backend
            .call("ctx_smells", serde_json::Value::Object(payload))
            .map_err(BridgeError::Backend)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    #[test]
    fn smells_is_registered() {
        assert!(super::super::default_registry().get("smells").is_some());
    }

    #[test]
    fn smells_unknown_action_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SmellsBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => {
                assert!(m.contains("unknown @smells action"), "got: {m}");
                assert!(
                    m.contains("Use: scan|summary|rules|file"),
                    "missing hint: {m}"
                );
            }
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn smells_rules_lists_the_ruleset() {
        // `rules` needs no fixture — it enumerates the static ruleset.
        let ctx = ctx_at(std::env::temp_dir());
        let out = SmellsBridge
            .execute(&ctx, &DirectiveArgs::parse("rules"))
            .unwrap();
        assert!(!out.trim().is_empty(), "empty @smells rules output");
    }

    #[test]
    fn smells_scan_dispatches_headless() {
        // scan builds the property graph over a temp fixture and renders findings
        // (or a "no smells" message) — either way a real, non-empty dispatch.
        let dir = std::env::temp_dir().join("lmd_smells_scan");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("s.rs"), "pub fn smell_anchor() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());
        let out = SmellsBridge
            .execute(&ctx, &DirectiveArgs::parse("scan"))
            .unwrap();
        assert!(!out.trim().is_empty(), "empty @smells scan output");
    }
}
