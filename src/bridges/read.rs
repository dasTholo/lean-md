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
        // Shared session cache from EngineContext — NOT SessionCache::new() per call.
        // Cold cache => always-full reads, never the ~13-tok re-read / auto-delta.
        // Re-reads stay cheap WITHOUT fresh/raw (spec §4.2a Read→Delta guarantee).
        let mut cache = ctx.cache.borrow_mut();
        let out = crate::tools::ctx_read::handle_with_task_resolved(
            &mut cache,
            path,
            mode,
            crate::crp_proto::CrpMode::Off,
            None,
        );
        if out.resolved_mode == "error" {
            return Err(BridgeError::Io(out.content));
        }
        Ok(out.content)
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
    fn reads_a_file_via_ctx_read() {
        let f = std::env::temp_dir().join("lmd_read_bridge.txt");
        std::fs::write(&f, "SENTINEL_LINE_42\n").unwrap();
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let args = DirectiveArgs::parse(f.to_str().unwrap());
        let out = ReadBridge.execute(&ctx, &args).unwrap();
        assert!(out.contains("SENTINEL_LINE_42"), "got: {out}");
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
