//! `@checkpoint` bridge -> shadow-git safety net via `ctx_checkpoint` (design section
//! "Neue Directive-Bridges"). Separate from the user's `.git`. Headless: outbound over the
//! CodeIntelBackend, BACKEND_REQUIRED envelope discarded by callers. `action`
//! (snapshot|log|diff|restore) is positional-0, default `snapshot`. Optional
//! `label=`/`message=`. Byte-stable (#498).

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct CheckpointBridge;

impl DirectiveBridge for CheckpointBridge {
    fn name(&self) -> &'static str {
        "checkpoint"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let action = match args
            .positional(0)
            .or_else(|| args.get("action"))
            .unwrap_or("snapshot")
        {
            a @ ("snapshot" | "log" | "diff" | "restore") => a,
            other => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @checkpoint action '{other}'. Use: snapshot|log|diff|restore"
                )));
            }
        };
        let mut payload = serde_json::Map::new();
        payload.insert("action".into(), action.into());
        if let Some(l) = args.get("label") {
            payload.insert("label".into(), l.into());
        }
        if let Some(m) = args.get("message") {
            payload.insert("message".into(), m.into());
        }
        let out = ctx
            .backend
            .call("ctx_checkpoint", serde_json::Value::Object(payload))
            .map_err(BridgeError::Backend)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;

    #[test]
    fn checkpoint_is_registered() {
        assert!(super::super::default_registry().get("checkpoint").is_some());
    }

    #[test]
    fn checkpoint_default_action_is_snapshot() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        // Empty args must default to `snapshot` (never hit the unknown-action
        // Resolve path) and dispatch outbound. Whether the real backend call
        // succeeds (dev env with a live `lean-ctx`) or fails at the process
        // level (lean-ctx unreachable) is environment-dependent — either
        // outcome is acceptable; only an unknown-action Resolve error is not.
        match CheckpointBridge.execute(&ctx, &DirectiveArgs::parse("")) {
            Ok(out) => assert!(!out.trim().is_empty(), "empty @checkpoint snapshot output"),
            Err(BridgeError::Backend(_)) => {}
            Err(other) => panic!("expected Ok or Backend error, got: {other:?}"),
        }
    }

    #[test]
    fn checkpoint_unknown_action_is_a_clear_error() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        let err = CheckpointBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => {
                assert!(m.contains("unknown @checkpoint action"), "got: {m}")
            }
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }
}
