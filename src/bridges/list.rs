//! `@list` Router bridge → the same core as the `ctx_tree` MCP tool.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@list [path] [depth=<n>]` — defaults path=".", depth=2, hidden files off,
/// gitignore respected. Routes `ctx_tree::handle`. No required arg.
pub struct ListBridge;

impl DirectiveBridge for ListBridge {
    fn name(&self) -> &'static str {
        "list"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let path = args
            .positional(0)
            .or_else(|| args.get("path"))
            .unwrap_or(".");
        let depth = args
            .get("depth")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(2);
        let mut payload = serde_json::Map::new();
        payload.insert("path".into(), path.into());
        payload.insert("depth".into(), (depth as u64).into());
        let out = ctx
            .backend
            .call("ctx_tree", serde_json::Value::Object(payload))
            .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}"));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn lists_a_directory_via_ctx_tree() {
        // @list routes outbound to ctx_tree. The dispatch contract: Ok(…) with
        // either the live tree (lean-ctx session present) or a BACKEND_REQUIRED
        // envelope (headless / jail-refused) — never Err, never a panic.
        let dir = std::env::temp_dir().join("lmd_list_bridge");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("listed_marker.txt"), "x\n").unwrap();
        let args = DirectiveArgs::parse(dir.to_str().unwrap());
        let out = ListBridge.execute(&ctx(), &args).unwrap();
        assert!(
            out.contains("listed_marker") || out.contains("BACKEND_REQUIRED"),
            "got: {out}"
        );
    }

    #[test]
    fn list_is_registered() {
        assert!(super::super::default_registry().get("list").is_some());
    }
}
