//! `@call name(args) /` bridge → macro expansion (spec §2.3, E-#4). Same
//! re-entry shape as `IncludeBridge`: look up the macro, substitute `{{ p }}`
//! params into its body, push the param scope (so `@if` conditions in the body
//! see them as evalexpr vars — Phase 4B), then `ctx.enter()` → `render_body` →
//! `leave()`. A passive (text) macro expands to markdown; an active (workflow)
//! macro whose body is `@reformat`/`@query`/… re-enters render_body and only
//! the dense result is emitted — the interna stay in the definition space.

use std::collections::HashMap;
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::{EngineContext, render_body};
use crate::macros::{parse_call_signature, substitute_params};

pub struct CallBridge;

impl DirectiveBridge for CallBridge {
    fn name(&self) -> &'static str {
        "call"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        // The block parser hands us the whole tail: `name(arg1, arg2) /`.
        let (macro_name, call_args) = parse_call_signature(args.raw())
            .ok_or_else(|| BridgeError::Resolve("malformed @call signature".to_string()))?;

        // Clone the def out of the RefCell so the borrow is released before the
        // re-entrant render_body (which mutates ctx.macros via extract).
        let def = ctx.macros.borrow().get(&macro_name).cloned();
        let Some(def) = def else {
            // No macro defined with this name — visible error (built-in-first).
            // Plugin-tool fallback is a lean-ctx core concept; lean-md routes
            // external tool calls via ctx.backend, not a local PluginManager.
            return Err(BridgeError::Resolve(format!("macro not found: {macro_name}")));
        };

        let expanded = substitute_params(&def.body, &def.params, &call_args);

        let mut scope = HashMap::new();
        for (i, p) in def.params.iter().enumerate() {
            scope.insert(p.clone(), call_args.get(i).cloned().unwrap_or_default());
        }
        // enter() BEFORE push_params: if the depth guard trips
        // (BridgeError::DepthExceeded), the `?` returns early and no param
        // scope was pushed — so the scope stack stays balanced. Pushing before
        // enter() would leak a frame on the DepthExceeded path (4A T3 review).
        ctx.enter()?;
        ctx.push_params(scope);
        let rendered = render_body(ctx, &expanded);
        ctx.leave();
        ctx.pop_params();

        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::render;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn call_is_registered() {
        assert!(super::super::default_registry().get("call").is_some());
    }

    #[test]
    fn passive_macro_expands_to_text_with_params() {
        let out =
            render("@define greet(name)\nHello {{ name }}!\n@define-end\n\n@call greet(World) /\n");
        assert!(out.contains("Hello World!"), "got: {out}");
        assert!(!out.contains("@define"), "definition leaked: {out}");
    }

    #[test]
    fn unknown_macro_is_a_visible_error_not_abort() {
        // No macro defined → visible "macro not found" error in the output.
        let out = render("@call nonexistent() /\n");
        assert!(
            out.contains("macro not found: nonexistent"),
            "must surface a visible resolve error, got: {out}"
        );
    }

    #[test]
    fn active_macro_reenters_and_dispatches_inner_directive() {
        // Active (workflow) macro: body is a directive. @count is headless +
        // deterministic → proves re-entry actually fires inner directives.
        let dir = std::env::temp_dir().join("lmd_call_active");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..2 {
            std::fs::write(dir.join(format!("a{i}.activ")), "x").unwrap();
        }
        let pattern = format!("{}/*.activ", dir.to_str().unwrap());
        let ctx = ctx();
        let out = crate::engine::render_body(
            &ctx,
            &format!("@define cnt()\n@count {pattern}\n@define-end\n\n@call cnt() /\n"),
        );
        assert!(
            out.contains('2'),
            "active macro must emit inner result: {out}"
        );
        assert!(
            !out.contains("@count"),
            "inner directive leaked verbatim: {out}"
        );
    }

    #[test]
    fn missing_arg_substitutes_empty() {
        let out = render("@define g(a)\n[{{ a }}]\n@define-end\n\n@call g() /\n");
        assert!(out.contains("[]"), "missing arg → empty, got: {out}");
    }

}
