//! `@include` extension bridge: resolve a fragment (built-in-first) and render
//! its content recursively (content visible), guarded by the engine depth chain.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::{EngineContext, render_body};
use crate::fragments::ResolveError;

pub struct IncludeBridge;

impl From<ResolveError> for BridgeError {
    fn from(e: ResolveError) -> Self {
        match e {
            ResolveError::NotFound(n) => BridgeError::Resolve(format!("fragment not found: {n}")),
            ResolveError::Jail(m) => BridgeError::Resolve(format!("jail: {m}")),
            ResolveError::Io(m) => BridgeError::Io(m),
        }
    }
}

impl DirectiveBridge for IncludeBridge {
    fn name(&self) -> &'static str {
        "include"
    }
    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let name = args.positional(0).ok_or(BridgeError::MissingArg("name"))?;
        let content = ctx.fragments.resolve(name, &ctx.jail_root)?;
        ctx.enter()?;
        let rendered = render_body(ctx, &content);
        ctx.leave();
        Ok(rendered)
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
    fn includes_builtin_fragment() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let out = IncludeBridge
            .execute(&ctx, &DirectiveArgs::parse("hard-rules"))
            .unwrap();
        assert!(
            out.contains("lean-ctx"),
            "included hard-rules must render; got: {out}"
        );
    }
    #[test]
    fn missing_name_errors() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let err = IncludeBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }
}
