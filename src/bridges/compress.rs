//! `@compress` bridge -> session-context checkpoint via `ctx_compress` (design section
//! "Neue Directive-Bridges", #541). For long conversations. `action` positional-0, default
//! `checkpoint`. Headless: outbound, BACKEND_REQUIRED envelope discarded. Byte-stable (#498).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct CompressBridge;

impl DirectiveBridge for CompressBridge {
    fn name(&self) -> &'static str {
        "compress"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let action = args
            .positional(0)
            .or_else(|| args.get("action"))
            .unwrap_or("checkpoint");
        let payload = serde_json::json!({ "action": action });
        let out = ctx
            .backend
            .call("ctx_compress", payload)
            .map_err(BridgeError::Backend)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;

    #[test]
    fn compress_is_registered() {
        assert!(super::super::default_registry().get("compress").is_some());
    }

    #[test]
    fn compress_default_action_reaches_backend() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        // Same environment-dependent dispatch as checkpoint's default-action test:
        // a live backend answers Ok, an unreachable one surfaces as Backend(_).
        match CompressBridge.execute(&ctx, &DirectiveArgs::parse("")) {
            Ok(out) => assert!(!out.trim().is_empty(), "empty @compress checkpoint output"),
            Err(BridgeError::Backend(_)) => {}
            Err(other) => panic!("expected Ok or Backend error, got: {other:?}"),
        }
    }
}
