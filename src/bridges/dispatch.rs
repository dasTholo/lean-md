//! `@dispatch phase=… role=… to_agent=…` bridge (Spec §3, Phase 7C). PURE
//! renderer (D-1): composes Dispatch-Contract (b) + phase-isolated content
//! (a, template-eager/work-lazy) + ToolSearch-Bootstrap (c). No spawn, no
//! ctx_agent/ctx_handoff call — the baton is instruction text in the contract.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::{EngineContext, render_body};

/// ToolSearch-Bootstrap (block c, Spec D-2/§3.2): loads the deferred lazy-core
/// tools before the first read in the subagent. Byte-stable (#498).
const BOOTSTRAP: &str = "## Bootstrap\nToolSearch(query=\"select:mcp__lean-ctx__ctx_read,mcp__lean-ctx__ctx_search,mcp__lean-ctx__ctx_shell,mcp__lean-ctx__ctx_edit,mcp__lean-ctx__ctx_tree\")\n";

/// Sentinel used when `to_agent` is absent so that `{{ controller_id }}` survives
/// `render_body` verbatim (render_body would evaluate the `{{ }}` template as an
/// unknown variable and destroy the placeholder). Byte-stable, parser-opaque.
const CONTROLLER_ID_SENTINEL: &str = "\x00CTRL_ID_PLACEHOLDER\x00";

/// Sentinel used for the user-supplied `to_agent` value. The raw value is injected
/// AFTER `render_body` runs so that user-controlled bytes (e.g. `{{ env.SECRET }}`)
/// are never re-parsed by the template engine (M-2: template injection guard).
const TO_AGENT_SENTINEL: &str = "\x00TO_AGENT_VALUE\x00";

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
        // Brief source (Spec Detail 3): exactly one of phase= OR skill=+companion=.
        let phase = args.get("phase").or_else(|| args.positional(0));
        let companion = args.get("companion");
        let raw_body: std::borrow::Cow<'_, str> = match (phase, companion) {
            (Some(_), Some(_)) => {
                return Err(BridgeError::Resolve(
                    "use exactly one of phase= or companion=".to_string(),
                ));
            }
            (Some(p), None) => {
                // (a) phase-isolated body — lookup in the capture pre-pass (C1).
                let Some(body) = ctx.phase_body(p) else {
                    return Ok(format!("<!-- lmd: PHASE_NOT_FOUND '{p}' -->\n"));
                };
                std::borrow::Cow::Owned(body)
            }
            (None, Some(c)) => {
                let skill = args.get("skill").ok_or(BridgeError::MissingArg("skill"))?;
                // (a') companion brief — embedded source, rendered work-lazy below.
                let Some(body) = crate::skills::companion_body(skill, c) else {
                    return Ok(format!("<!-- lmd: COMPANION_NOT_FOUND '{skill}/{c}' -->\n"));
                };
                std::borrow::Cow::Borrowed(body)
            }
            (None, None) => return Err(BridgeError::MissingArg("phase")),
        };

        // role: dev|review|test, default dev.
        let role = match args.get("role") {
            Some(r @ ("dev" | "review" | "test")) => r,
            Some(other) => {
                return Err(BridgeError::Resolve(format!(
                    "unknown @dispatch role '{other}'. Use: dev|review|test"
                )));
            }
            None => "dev",
        };

        // (b) Contract: substitute placeholders using sentinels BEFORE render_body so
        // that user-controlled bytes never enter the template parser (M-2 guard).
        let contract_raw = ctx
            .fragments
            .resolve("dispatch-contract", &ctx.jail_root)
            .map_err(|_| BridgeError::Resolve("CONTRACT_UNAVAILABLE".to_string()))?;
        // crp is a controlled enum value (off|compact|tdd) — safe to inline
        // before render_body (no user bytes, no template-injection risk).
        let crp_str = match ctx.header.crp {
            crate::crp_proto::CrpMode::Off => "off",
            crate::crp_proto::CrpMode::Compact => "compact",
            crate::crp_proto::CrpMode::Tdd => "tdd",
        };
        // role is a validated enum value (dev|review) — safe to inline before render.
        let mut contract = contract_raw
            .replace("{{ role }}", role)
            .replace("{{ crp }}", crp_str);
        let mut warning = String::new();
        let to_agent_restore: &str;
        // Both branches replace {{ controller_id }} with a parser-opaque sentinel.
        // The real value (or the literal placeholder) is restored AFTER render_body,
        // so user-supplied bytes in to_agent are never evaluated by the template engine.
        let missing_to_agent = if let Some(_id) = args.get("to_agent") {
            // Guard: substitute a sentinel — NOT the raw user value — before render.
            contract = contract.replace("{{ controller_id }}", TO_AGENT_SENTINEL);
            to_agent_restore = _id; // restore real value after render
            false
        } else {
            warning.push_str(
                "<!-- lmd: WARNING @dispatch to_agent missing; baton placeholder kept -->\n",
            );
            // Replace {{ controller_id }} with a parser-opaque sentinel so that
            // render_body does not evaluate and destroy the placeholder.
            contract = contract.replace("{{ controller_id }}", CONTROLLER_ID_SENTINEL);
            to_agent_restore = ""; // unused in this branch
            true
        };
        let mut contract_rendered = render_body(ctx, &contract); // resolves @include hard-rules
        if missing_to_agent {
            // Restore the human-visible placeholder text.
            contract_rendered =
                contract_rendered.replace(CONTROLLER_ID_SENTINEL, "{{ controller_id }}");
        } else {
            // Restore the real to_agent value — injected AFTER render_body (M-2 safe).
            contract_rendered = contract_rendered.replace(TO_AGENT_SENTINEL, to_agent_restore);
        }

        // (a) template-eager / work-lazy render of the phase body (C2).
        let body_rendered = crate::render::splice_template_only(ctx, &raw_body);

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
    use crate::engine::render;

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

    #[test]
    fn invalid_role_is_rejected() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=admin to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("unknown @dispatch role") || out.contains("admin"),
            "invalid role must surface an error: {out}"
        );
    }

    #[test]
    fn rendered_prompt_carries_native_read_prohibition() {
        // §8.8 (Rust part): the rendered prompt itself prohibits native I/O —
        // the in-prompt discipline that steers the subagent to ctx_*.
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("ctx_read"),
            "prompt must steer to ctx_read: {out}"
        );
        assert!(
            out.contains("never cat") || out.contains("never grep/rg") || out.contains("NEVER"),
            "prompt must prohibit native I/O: {out}"
        );
    }

    /// M-2: A to_agent value containing an inline template directive must appear
    /// LITERALLY in the output — the template engine must NOT evaluate it.
    #[test]
    fn to_agent_template_injection_is_neutralized() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"{{ env.HOME }}\"\n";
        let out = render(doc);
        // The literal string must appear verbatim.
        assert!(
            out.contains("to_agent={{ env.HOME }}"),
            "to_agent injection must appear literally, not expanded: {out}"
        );
        // The expanded home path must NOT be present as the to_agent value.
        // (We check that the env HOME value, if set, is not the substituted result.)
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            assert!(
                !out.contains(&format!("to_agent={home}")),
                "expanded HOME must not appear as to_agent value: {out}"
            );
        }
    }

    #[test]
    fn dispatch_threads_crp_tdd_into_contract() {
        let doc = "@lean-md\ncrp: tdd\n\n@phase \"P\"\nDo the work.\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("CRP mode `tdd`"), "crp threaded: {out}");
        assert!(!out.contains("{{ crp }}"), "placeholder substituted: {out}");
    }

    #[test]
    fn dispatch_threads_crp_compact_into_contract() {
        let doc = "@lean-md\ncrp: compact\n\n@phase \"P\"\nDo the work.\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("CRP mode `compact`"),
            "crp compact threaded: {out}"
        );
        assert!(!out.contains("{{ crp }}"), "placeholder substituted: {out}");
    }

    #[test]
    fn dispatch_threads_crp_off_by_default() {
        let doc = "@phase \"P\"\nDo the work.\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("CRP mode `off`"),
            "default off threaded: {out}"
        );
    }

    /// M-3: The rendered output must never contain a raw NUL byte — the sentinel
    /// must always be fully restored for both present- and missing-to_agent paths.
    #[test]
    fn rendered_output_contains_no_nul_bytes() {
        // Path 1: to_agent present.
        let doc_present = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"ctrl-1\"\n";
        let out_present = render(doc_present);
        assert!(
            !out_present.contains('\u{0}'),
            "NUL byte leaked in present-to_agent output: {out_present:?}"
        );

        // Path 2: to_agent absent.
        let doc_absent = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev\n";
        let out_absent = render(doc_absent);
        assert!(
            !out_absent.contains('\u{0}'),
            "NUL byte leaked in missing-to_agent output: {out_absent:?}"
        );
    }

    #[test]
    fn dispatch_companion_brief_composes_contract_methodology_bootstrap() {
        let doc = "@dispatch skill=\"lmd-writing-skills\" companion=\"testing/methodology\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(out.contains("role=test"), "test role missing: {out}");
        assert!(
            out.contains("RED Phase"),
            "methodology marker missing: {out}"
        );
        assert!(
            out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "Iron Law via @include missing: {out}"
        );
        assert!(
            out.contains("ToolSearch(query=\"select:mcp__lean-ctx__ctx_read"),
            "bootstrap missing: {out}"
        );
    }

    #[test]
    fn dispatch_phase_source_still_works() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=dev to_agent=\"c\"\n";
        let out = render(doc);
        assert!(out.contains("role=dev"), "phase path regressed: {out}");
        assert!(out.contains("@read a.rs"), "work directive verbatim: {out}");
    }

    #[test]
    fn dispatch_rejects_both_phase_and_companion() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" skill=\"lmd-writing-skills\" companion=\"testing/methodology\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("exactly one of phase= or companion="),
            "both-given must Resolve-error: {out}"
        );
    }

    #[test]
    fn dispatch_companion_requires_skill() {
        let doc = "@dispatch companion=\"testing/methodology\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("skill") && (out.contains("MissingArg") || out.contains("missing")),
            "companion= without skill= must MissingArg(skill): {out}"
        );
    }

    #[test]
    fn dispatch_unknown_companion_yields_envelope() {
        let doc =
            "@dispatch skill=\"lmd-writing-skills\" companion=\"nope\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("COMPANION_NOT_FOUND 'lmd-writing-skills/nope'"),
            "unknown companion must yield envelope, not abort: {out}"
        );
    }

    #[test]
    fn dispatch_test_role_substitutes() {
        let doc = "@phase \"P\"\n@read a.rs\n@phase-end\n\n@dispatch phase=\"P\" role=test to_agent=\"c\"\n";
        let out = render(doc);
        assert!(
            out.contains("role=test"),
            "test role must substitute: {out}"
        );
    }
}
