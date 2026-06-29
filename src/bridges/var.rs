//! `@var` bridge — skill-body variables (Spec: @var + .lean-ctx/lean-md/vars.toml).
//! Dual-role like `@include`: a block declaration `@var NAME default="…" desc="…"`
//! registers a default (config-precedence) and renders empty; the inline
//! `{{ var NAME }}` looks the value up. Template for this bridge: `env.rs`.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@var NAME default="…" [desc="…"]` (declaration) / `{{ var NAME }}` (lookup).
pub struct VarBridge;

impl DirectiveBridge for VarBridge {
    fn name(&self) -> &'static str {
        "var"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let name = args.positional(0).ok_or(BridgeError::MissingArg("name"))?;
        match args.get("default") {
            // Declaration mode: set default only if absent (config wins), render empty.
            Some(default) => {
                ctx.var_set_default(name, default);
                Ok(String::new())
            }
            // Lookup mode: resolved value, or empty if unknown (author error, no panic).
            None => Ok(ctx.var_get(name).unwrap_or_default()),
        }
    }
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
    fn declaration_renders_empty_and_sets_default() {
        let ctx = ctx();
        let out = VarBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(r#"test_cmd default="cargo test""#),
            )
            .unwrap();
        assert_eq!(out, "");
        assert_eq!(ctx.var_get("test_cmd"), Some("cargo test".to_string()));
    }

    #[test]
    fn declaration_does_not_override_config() {
        let ctx = ctx();
        let mut m = std::collections::HashMap::new();
        m.insert("test_cmd".to_string(), "cargo nextest run".to_string());
        ctx.vars_seed(m);
        VarBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(r#"test_cmd default="cargo test""#),
            )
            .unwrap();
        assert_eq!(
            ctx.var_get("test_cmd"),
            Some("cargo nextest run".to_string())
        );
    }

    #[test]
    fn lookup_returns_value() {
        let ctx = ctx();
        ctx.var_set_default("test_cmd", "cargo test");
        let out = VarBridge
            .execute(&ctx, &DirectiveArgs::parse("test_cmd"))
            .unwrap();
        assert_eq!(out, "cargo test");
    }

    #[test]
    fn lookup_unknown_is_empty() {
        let out = VarBridge
            .execute(&ctx(), &DirectiveArgs::parse("nope"))
            .unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn missing_name_errors() {
        let err = VarBridge
            .execute(&ctx(), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn var_is_registered() {
        assert!(super::super::default_registry().get("var").is_some());
    }
}
