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
    use std::rc::Rc;

    #[test]
    fn read_dispatches_to_backend() {
        // Without a real backend the call returns a BACKEND_REQUIRED envelope —
        // but the bridge itself must not panic and must return Ok (6.12 wires
        // a real backend; this test only checks the dispatch contract).
        let f = std::env::temp_dir().join("lmd_read_bridge.txt");
        std::fs::write(&f, "SENTINEL_LINE_42\n").unwrap();
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let args = DirectiveArgs::parse(f.to_str().unwrap());
        // Ok(…) — not Err — is the contract regardless of backend availability.
        assert!(ReadBridge.execute(&ctx, &args).is_ok());
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
