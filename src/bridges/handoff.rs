//! `@handoff` bridge → Context Ledger Protocol (Spec §4, Phase 7B). Orthogonal
//! zu `@dispatch` (D-1): explizite, durable Bundle-Direktive. Routet direkt auf
//! `core::handoff_ledger` (NICHT den ctx_handoff-McpTool — der braucht ToolContext,
//! den die lmd-Engine nicht hat). Sinks-gated: headless ⇒ deterministischer No-op.

use std::path::Path;
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct HandoffBridge;

impl DirectiveBridge for HandoffBridge {
    fn name(&self) -> &'static str {
        "handoff"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let action = args
            .positional(0)
            .or_else(|| args.get("action"))
            .unwrap_or("create");
        match action {
            "create" => handoff_create(ctx),
            "show" => handoff_show(ctx, args),
            "pull" => handoff_pull(ctx, args),
            other => Err(BridgeError::Resolve(format!(
                "unknown @handoff action '{other}'. Use: create|show|pull"
            ))),
        }
    }
}

/// `@handoff create` → durables Ledger-Bundle aus der aktuellen Session.
/// Sinks-gated: kein Sink ⇒ leerer No-op (headless-deterministisch, #498).
/// tool_calls/workflow/curated_refs sind in der lmd-Engine nicht verfügbar →
/// leer; das Bundle trägt den Session-Snapshot (Task/Decisions/Findings).
fn handoff_create(ctx: &Rc<EngineContext>) -> Result<String, BridgeError> {
    let Some(sinks) = ctx.sinks.as_ref() else {
        return Ok(String::new()); // headless: kein Bundle-Write
    };
    let Some(session_lock) = sinks.session.as_ref() else {
        return Ok(String::new());
    };
    let session = session_lock.blocking_read().clone();
    let project_root = session.project_root.clone();
    let (ledger, path) = crate::core::handoff_ledger::create_ledger(
        crate::core::handoff_ledger::CreateLedgerInput {
            agent_id: sinks.agent_id.clone(),
            client_name: None,
            project_root,
            session,
            tool_calls: Vec::new(),
            workflow: None,
            curated_refs: Vec::new(),
        },
    )
    .map_err(BridgeError::Io)?;
    Ok(crate::tools::ctx_handoff::format_created(&path, &ledger))
}

/// `@handoff show path=<ledger>` → Read-only Render eines Bundles. Pfad wird
/// gegen den Jail-Root aufgelöst (PathJail erbt, Spec §7).
fn handoff_show(ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
    let raw = args
        .get("path")
        .or_else(|| args.positional(1))
        .ok_or(BridgeError::MissingArg("path"))?;
    let path = resolve_jailed(ctx, raw)?;
    let ledger = crate::core::handoff_ledger::load_ledger(&path).map_err(BridgeError::Io)?;
    Ok(crate::tools::ctx_handoff::format_show(&path, &ledger))
}

/// `@handoff pull path=<ledger>` → Bundle laden und Session-Snapshot anwenden
/// (Task/Decisions/Findings/next_steps) über das Sink-Session-Handle. Sinks-gated.
fn handoff_pull(ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
    let raw = args
        .get("path")
        .or_else(|| args.positional(1))
        .ok_or(BridgeError::MissingArg("path"))?;
    let path = resolve_jailed(ctx, raw)?;
    let ledger = crate::core::handoff_ledger::load_ledger(&path).map_err(BridgeError::Io)?;
    let Some(sinks) = ctx.sinks.as_ref() else {
        return Ok(String::new()); // headless: nichts anzuwenden
    };
    let Some(session_lock) = sinks.session.as_ref() else {
        return Ok(String::new());
    };
    {
        let mut session = session_lock.blocking_write();
        if let Some(t) = ledger.session.task.as_deref() {
            session.set_task(t, None);
        }
        for d in &ledger.session.decisions {
            session.add_decision(d, None);
        }
        for f in &ledger.session.findings {
            session.add_finding(None, None, f);
        }
        session.next_steps.clone_from(&ledger.session.next_steps);
        session.save().map_err(BridgeError::Io)?;
    }
    Ok(format!(
        "ctx_handoff pull\n path: {}\n md5: {}\n",
        path.display(),
        ledger.content_md5
    ))
}

/// Jail-Resolve eines Ledger-Pfads relativ zum Engine-Jail-Root. Verbietet
/// ParentDir/RootDir-Escape vor jedem FS-Zugriff (Muster aus fragments.rs).
fn resolve_jailed(ctx: &Rc<EngineContext>, raw: &str) -> Result<std::path::PathBuf, BridgeError> {
    use std::path::Component;
    if Path::new(raw)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(BridgeError::Resolve(format!("'{raw}' escapes jail")));
    }
    let candidate = if Path::new(raw).is_absolute() {
        std::path::PathBuf::from(raw)
    } else {
        ctx.jail_root.join(raw)
    };
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn headless_ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn unknown_action_is_a_clear_error() {
        let ctx = headless_ctx();
        let err = HandoffBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @handoff action"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn headless_create_is_deterministic_noop() {
        // No sinks ⇒ no ledger write, empty deterministic output (#498).
        let ctx = headless_ctx();
        let out = HandoffBridge
            .execute(&ctx, &DirectiveArgs::parse("create"))
            .unwrap();
        assert_eq!(out, "", "headless @handoff create must render nothing");
    }

    #[test]
    fn show_missing_path_errors() {
        let ctx = headless_ctx();
        let err = HandoffBridge
            .execute(&ctx, &DirectiveArgs::parse("show"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("path")),
            "got: {err:?}"
        );
    }

    #[test]
    fn handoff_is_registered() {
        assert!(super::super::default_registry().get("handoff").is_some());
    }
}
