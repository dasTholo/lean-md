//! `@call name(args) /` bridge → macro expansion (spec §2.3, E-#4). Same
//! re-entry shape as `IncludeBridge`: look up the macro, substitute `{{ p }}`
//! params into its body, push the param scope (so `@if` conditions in the body
//! see them as evalexpr vars — Phase 4B), then `ctx.enter()` → `render_body` →
//! `leave()`. A passive (text) macro expands to markdown; an active (workflow)
//! macro whose body is `@reformat`/`@query`/… re-enters render_body and only
//! the dense result is emitted — the interna stay in the definition space.

use std::collections::HashMap;
use std::rc::Rc;

use serde_json::Value;

use super::{BridgeError, DirectiveBridge};
use crate::core::plugins::PluginManager;
use crate::core::plugins::tools::{PluginToolSpec, invoke};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::{EngineContext, render_body};
use crate::lmd::macros::{parse_call_signature, substitute_params};

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
            // Built-in macro missed → plugin-tool fallback (gated). Built-in-first:
            // an authored @define of the same name always won above.
            return resolve_plugin_call(ctx, &macro_name, &call_args);
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

/// Built-in macro missed → resolve `name` against the enabled plugins' `[[tools]]`.
/// Gated behind `@lean-md extensions=allow`; sandbox/trust are inherited from
/// `tools::invoke` (env-scrub + cwd-jail + timeout) — no new exec guard here.
fn resolve_plugin_call(
    ctx: &Rc<EngineContext>,
    name: &str,
    call_args: &[String],
) -> Result<String, BridgeError> {
    if !ctx.header.extensions_allowed() {
        return Err(BridgeError::Resolve(format!(
            "@call '{name}': plugin tools disabled (set `@lean-md extensions=allow`)"
        )));
    }
    let Some(spec) = plugin_tool_spec(name) else {
        // No macro, no plugin tool → same visible error as before (built-in-first).
        return Err(BridgeError::Resolve(format!("macro not found: {name}")));
    };
    let required = schema_required(&spec.input_schema);
    if call_args.len() < required.len() {
        return Err(BridgeError::Resolve(format!(
            "@call '{name}': missing argument(s) — schema requires {}, got {}",
            required.len(),
            call_args.len()
        )));
    }
    let args_json = call_args_to_json(&schema_param_order(&spec.input_schema), call_args);
    invoke(&spec, &args_json).map_err(|e| BridgeError::Resolve(format!("@call '{name}': {e}")))
}

fn plugin_tool_spec(name: &str) -> Option<PluginToolSpec> {
    PluginManager::tool_specs()
        .into_iter()
        .find(|s| s.name == name)
}

/// Ordered required property names (a JSON array preserves authored order even
/// though serde_json's object Map is a BTreeMap without `preserve_order`).
fn schema_required(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Positional → key order: the `required` array if present, else `properties` keys.
fn schema_param_order(schema: &Value) -> Vec<String> {
    let req = schema_required(schema);
    if !req.is_empty() {
        return req;
    }
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Map positional `@call name(a, b)` args onto a JSON object by schema order.
fn call_args_to_json(order: &[String], args: &[String]) -> String {
    let mut obj = serde_json::Map::new();
    for (k, v) in order.iter().zip(args.iter()) {
        obj.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(obj).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::engine::render;
    use crate::lmd::header::LeanMdHeader;
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
        // Default header → extensions=deny. The deny gate fires before the tool
        // lookup, so the error reports the gate (not "macro not found").
        let out = render("@call nonexistent() /\n");
        assert!(
            out.contains("plugin tools disabled"),
            "must surface a visible resolve error, got: {out}"
        );
    }

    #[test]
    fn unknown_macro_with_extensions_allow_says_macro_not_found() {
        // extensions=allow but no plugin tool with this name → falls through to
        // the "macro not found" error (built-in-first invariant).
        let out = render("@lean-md\nextensions: allow\n\n@call nonexistent() /\n");
        assert!(
            out.contains("macro not found: nonexistent"),
            "no plugin → macro-not-found error, got: {out}"
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
        let out = crate::lmd::engine::render_body(
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

    fn install_echo_plugin() -> std::path::PathBuf {
        // LEAN_CTX_PLUGINS_DIR root → <root>/echo-plugin/plugin.toml. cat echoes
        // stdin (= the args_json) so the smoke assertion is deterministic.
        let root = std::env::temp_dir().join("lmd_p5_call_plugins");
        let pdir = root.join("echo-plugin");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(
            pdir.join("plugin.toml"),
            r#"[plugin]
name = "echo-plugin"
version = "0.1.0"

[[tools]]
name = "stub"
description = "echo stdin"
command = "cat"
input_schema = { type = "object", properties = { text = { type = "string" } }, required = ["text"] }
"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn call_plugin_denied_without_extensions_allow() {
        let root = install_echo_plugin();
        crate::test_env::set_var("LEAN_CTX_PLUGINS_DIR", root.to_str().unwrap());
        crate::core::plugins::PluginManager::init();
        // Default header => extensions=deny.
        let out = render("@call stub(hello) /\n");
        crate::test_env::remove_var("LEAN_CTX_PLUGINS_DIR");
        assert!(
            out.contains("plugin tools disabled"),
            "deny-by-default gate must fire, got: {out}"
        );
        assert!(
            !out.contains("\"text\""),
            "no subprocess may run when gated: {out}"
        );
    }

    #[test]
    fn call_plugin_fires_with_extensions_allow() {
        let root = install_echo_plugin();
        crate::test_env::set_var("LEAN_CTX_PLUGINS_DIR", root.to_str().unwrap());
        crate::core::plugins::PluginManager::init();
        // extensions=allow → cat echoes the mapped args_json {"text":"hello"}.
        let out = render("@lean-md\nextensions: allow\n\n@call stub(hello) /\n");
        crate::test_env::remove_var("LEAN_CTX_PLUGINS_DIR");
        assert!(out.contains("hello"), "plugin stdout must surface: {out}");
        assert!(
            !out.contains("plugin tools disabled"),
            "gate must be open: {out}"
        );
    }

    #[test]
    fn builtin_macro_wins_over_plugin_tool_of_same_name() {
        let root = install_echo_plugin();
        crate::test_env::set_var("LEAN_CTX_PLUGINS_DIR", root.to_str().unwrap());
        crate::core::plugins::PluginManager::init();
        // A @define 'stub' must win over the plugin tool 'stub' (built-in-first),
        // even with the gate open — the plugin (cat) must never run.
        let out = render(
            "@lean-md\nextensions: allow\n\n@define stub()\nMACRO_WINS\n@define-end\n\n@call stub() /\n",
        );
        crate::test_env::remove_var("LEAN_CTX_PLUGINS_DIR");
        assert!(out.contains("MACRO_WINS"), "macro must win: {out}");
        assert!(!out.contains("\"text\""), "plugin must not run: {out}");
    }
}
