//! `@dispatch phase=… role=… to_agent=…` bridge (Spec §3, Phase 7C). REINER
//! Renderer (D-1): komponiert Dispatch-Contract (b) + phasen-isolierten Inhalt
//! (a, template-eager/work-lazy) + ToolSearch-Bootstrap (c). Kein Spawn, kein
//! ctx_agent/ctx_handoff-Aufruf — der Baton ist Instruktions-Text im Contract.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::{EngineContext, render_body};

/// ToolSearch-Bootstrap (Block c, Spec D-2/§3.2): lädt die deferred lazy-core-
/// Tools vor dem ersten Read im Subagenten. Byte-stabil (#498).
const BOOTSTRAP: &str = "## Bootstrap\nToolSearch(query=\"select:mcp__lean-ctx__ctx_read,mcp__lean-ctx__ctx_search,mcp__lean-ctx__ctx_shell,mcp__lean-ctx__ctx_edit,mcp__lean-ctx__ctx_tree\")\n";

/// Sentinel used when `to_agent` is absent so that `{{ controller_id }}` survives
/// `render_body` verbatim (render_body would evaluate the `{{ }}` template as an
/// unknown variable and destroy the placeholder). Byte-stable, parser-opaque.
const CONTROLLER_ID_SENTINEL: &str = "\x00CTRL_ID_PLACEHOLDER\x00";

pub struct DispatchBridge;

impl DirectiveBridge for DispatchBridge {
    fn name(&self) -> &'static str {
        "dispatch"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let phase = args
            .get("phase")
            .or_else(|| args.positional(0))
            .ok_or(BridgeError::MissingArg("phase"))?;

        // (a) phasen-isolierter Body — Lookup im capture-Pre-Pass (C1).
        let Some(raw_body) = ctx.phase_body(phase) else {
            return Ok(format!("<!-- lmd: PHASE_NOT_FOUND '{phase}' -->\n"));
        };

        // role: dev|review, default dev.
        let role = match args.get("role") {
            Some(r @ ("dev" | "review")) => r,
            Some(other) => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @dispatch role '{other}'. Use: dev|review"
                )));
            }
            None => "dev",
        };

        // (b) Contract: substitute placeholders BEFORE render, then resolve @include.
        let contract_raw = ctx
            .fragments
            .resolve("dispatch-contract", &ctx.jail_root)
            .map_err(|_| BridgeError::Resolve("CONTRACT_UNAVAILABLE".to_string()))?;
        let mut contract = contract_raw.replace("{{ role }}", role);
        let mut warning = String::new();
        let missing_to_agent = match args.get("to_agent") {
            Some(id) => {
                contract = contract.replace("{{ controller_id }}", id);
                false
            }
            None => {
                warning.push_str(
                    "<!-- lmd: WARNING @dispatch to_agent missing; baton placeholder kept -->\n",
                );
                // Replace {{ controller_id }} with a parser-opaque sentinel so that
                // render_body does not evaluate and destroy the placeholder. We restore
                // the literal {{ controller_id }} in the output afterward.
                contract = contract.replace("{{ controller_id }}", CONTROLLER_ID_SENTINEL);
                true
            }
        };
        let mut contract_rendered = render_body(ctx, &contract); // resolves @include hard-rules
        if missing_to_agent {
            // Restore the human-visible placeholder text.
            contract_rendered =
                contract_rendered.replace(CONTROLLER_ID_SENTINEL, "{{ controller_id }}");
        }

        // (a) template-eager / work-lazy render of the phase body (C2).
        let body_rendered = crate::lmd::render::splice_template_only(ctx, &raw_body);

        // Compose (b) + (a) + (c). Stable headers (#498).
        let mut out = String::new();
        out.push_str(&warning);
        out.push_str(&contract_rendered);
        out.push_str("\n## Task (phase-isolated)\n");
        out.push_str(&body_rendered);
        if !body_rendered.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(BOOTSTRAP);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::lmd::engine::render;

    #[test]
    fn dispatch_is_registered() {
        assert!(super::super::default_registry().get("dispatch").is_some());
    }

    #[test]
    fn unknown_phase_yields_phase_not_found_envelope() {
        let out = render("@dispatch phase=\"nope\"\n");
        assert!(out.contains("PHASE_NOT_FOUND"), "got: {out}");
    }

    #[test]
    fn composes_contract_body_and_bootstrap_with_work_lazy() {
        let doc = "\
@phase \"A3\"
@read src/x.rs
@query \"cargo nextest run\"
@phase-end

@dispatch phase=\"A3\" role=dev to_agent=\"ctrl-1\"
";
        let out = render(doc);
        // (b) contract + discipline:
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(out.contains("role=dev"), "role substitution missing: {out}");
        assert!(
            out.contains("to_agent=ctrl-1"),
            "controller_id substitution missing: {out}"
        );
        // (a) work bridges verbatim (lazy):
        assert!(
            out.contains("@read src/x.rs"),
            "work @read must stay verbatim: {out}"
        );
        assert!(
            out.contains("## Task (phase-isolated)"),
            "task header missing: {out}"
        );
        // (c) bootstrap:
        assert!(
            out.contains("ToolSearch(query=\"select:mcp__lean-ctx__ctx_read"),
            "bootstrap missing: {out}"
        );
    }

    #[test]
    fn review_role_substitutes() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=review to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("role=review"), "got: {out}");
    }

    #[test]
    fn missing_to_agent_warns_but_does_not_abort() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev\n";
        let out = render(doc);
        assert!(out.contains("WARNING"), "missing to_agent must warn: {out}");
        assert!(
            out.contains("{{ controller_id }}"),
            "placeholder kept visible: {out}"
        );
        assert!(
            out.contains("@read a.rs"),
            "render must still produce the prompt: {out}"
        );
    }
}
