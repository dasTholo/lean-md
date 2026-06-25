//! `@read` Router bridge → the same core as the `ctx_read` MCP tool.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@read <path> [mode=<mode>]` — defaults to `auto`. Phase 1 passes the path
/// through unchanged (jailing `@read` is a §7/Phase-7 concern).
pub struct ReadBridge;

impl DirectiveBridge for ReadBridge {
    fn name(&self) -> &'static str {
        "read"
    }
    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let path = args
            .positional(0)
            .or_else(|| args.get("path"))
            .ok_or(BridgeError::MissingArg("path"))?;
        let mode = args.get("mode").unwrap_or("auto");
        let out = ctx
            .backend
            .call(
                "ctx_read",
                serde_json::json!({ "path": path, "mode": mode }),
            )
            .map_err(BridgeError::Backend)?;
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
    use std::rc::Rc;

    #[test]
    fn read_dispatches_to_backend() {
        // Post-I2 dispatch contract: a successful backend exit-0 yields Ok (live
        // content or a tool-owned envelope); a real backend failure (lean-ctx
        // absent / jail-refused) yields Err(BridgeError::Backend) so a @read
        // inside a @phase aborts. The bridge must never panic.
        let f = std::env::temp_dir().join("lmd_read_bridge.txt");
        std::fs::write(&f, "SENTINEL_LINE_42\n").unwrap();
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let args = DirectiveArgs::parse(f.to_str().unwrap());
        match ReadBridge.execute(&ctx, &args) {
            Ok(_) => {}
            Err(BridgeError::Backend(_)) => {}
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
    #[test]
    fn missing_path_errors() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let err = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }
}
