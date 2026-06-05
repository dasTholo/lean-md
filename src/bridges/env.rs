//! `@env` Router bridge → process environment lookup (Spec §3.1 trivial-R).
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

/// `@env <KEY>` — expands to the value of env var `KEY`, or empty if unset.
pub struct EnvBridge;

impl DirectiveBridge for EnvBridge {
    fn name(&self) -> &'static str {
        "env"
    }

    fn execute(
        &self,
        _ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let key = args.positional(0).ok_or(BridgeError::MissingArg("key"))?;
        Ok(std::env::var(key).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::args::DirectiveArgs;
    use crate::lmd::engine::EngineContext;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn expands_set_var() {
        std::env::set_var("LMD_ENV_TEST_KEY", "env_marker_7");
        let out = EnvBridge
            .execute(&ctx(), &DirectiveArgs::parse("LMD_ENV_TEST_KEY"))
            .unwrap();
        assert_eq!(out, "env_marker_7");
    }

    #[test]
    fn unset_var_is_empty() {
        let out = EnvBridge
            .execute(&ctx(), &DirectiveArgs::parse("LMD_DEFINITELY_UNSET_XYZ"))
            .unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn missing_key_errors() {
        let err = EnvBridge
            .execute(&ctx(), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn env_is_registered() {
        assert!(super::super::default_registry().get("env").is_some());
    }
}
