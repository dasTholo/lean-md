//! `@handoff` bridge → Context Ledger Protocol (Spec §4, Phase 7B). Orthogonal
//! zu `@dispatch` (D-1): explizite, durable Bundle-Direktive. Routes outbound
//! via `ctx.backend.call("ctx_handoff", …)` — no local handoff_ledger access.

use std::rc::Rc;

use serde_json::json;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

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

/// `@handoff create` → durables Ledger-Bundle via outbound ctx_handoff call.
/// Routes to `ctx.backend.call("ctx_handoff", {"action":"create"})`.
fn handoff_create(ctx: &Rc<EngineContext>) -> Result<String, BridgeError> {
    ctx.backend
        .call("ctx_handoff", json!({"action": "create"}))
        .map_err(BridgeError::Backend)
}

/// `@handoff show path=<ledger>` → Read-only Render eines Bundles. Pfad wird
/// gegen den Jail-Root aufgelöst (PathJail erbt, Spec §7), dann outbound.
fn handoff_show(ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
    let raw = args
        .get("path")
        .or_else(|| args.positional(1))
        .ok_or(BridgeError::MissingArg("path"))?;
    let path = resolve_jailed(ctx, raw)?;
    let path_str = path.to_string_lossy();
    ctx.backend
        .call(
            "ctx_handoff",
            json!({"action": "show", "path": path_str.as_ref()}),
        )
        .map_err(BridgeError::Backend)
}

/// `@handoff pull path=<ledger>` → Bundle laden und Session-Snapshot anwenden
/// via outbound ctx_handoff call.
fn handoff_pull(ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
    let raw = args
        .get("path")
        .or_else(|| args.positional(1))
        .ok_or(BridgeError::MissingArg("path"))?;
    let path = resolve_jailed(ctx, raw)?;
    let path_str = path.to_string_lossy();
    ctx.backend
        .call(
            "ctx_handoff",
            json!({"action": "pull", "path": path_str.as_ref()}),
        )
        .map_err(BridgeError::Backend)
}

/// Jail-Resolve eines Ledger-Pfads relativ zum Engine-Jail-Root.
///
/// Delegates to `crate::pathx::jail_path` — the canonical path jail
/// (null-byte rejection, `path_jail=false` config bypass, session extra_roots
/// via #403, not-yet-existing paths via `canonicalize_existing_ancestor`).
/// An absolute `raw` makes `join` return `raw` itself (Rust path semantics),
/// so `jail_path` is what enforces the boundary for both relative and absolute
/// inputs — identical to the graph.rs §7 idiom.
fn resolve_jailed(ctx: &Rc<EngineContext>, raw: &str) -> Result<std::path::PathBuf, BridgeError> {
    let candidate = ctx.jail_root.join(raw);
    crate::pathx::jail_path(&candidate, &ctx.jail_root).map_err(|e| {
        // Normalise to "escapes jail" wording so callers get a consistent message.
        BridgeError::Resolve(format!("'{raw}' escapes jail: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
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
    fn headless_create_returns_backend_envelope_or_empty() {
        // Post-I2: `@handoff create` routes outbound to ctx_handoff. A real
        // backend failure (lean-ctx absent / jail-refused) is now
        // Err(BridgeError::Backend) so it aborts an enclosing @phase; an exit-0
        // backend yields Ok(empty | tool-owned envelope). Never a panic.
        let ctx = headless_ctx();
        match HandoffBridge.execute(&ctx, &DirectiveArgs::parse("create")) {
            Ok(out) => assert!(
                out.is_empty() || out.contains("BACKEND") || out.contains("ERROR"),
                "exit-0 @handoff create must be empty or a tool envelope, got: {out:?}"
            ),
            Err(BridgeError::Backend(_)) => {}
            Err(other) => panic!("unexpected error: {other:?}"),
        }
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

    #[test]
    fn resolve_jailed_blocks_absolute_path_escape() {
        // Absolute paths must NOT escape the jail — e.g. /etc/passwd must Err.
        let ctx = headless_ctx();
        let result = resolve_jailed(&ctx, "/etc/passwd");
        assert!(
            result.is_err(),
            "absolute path jail escape must return Err, got Ok({:?})",
            result.ok()
        );
        match result.unwrap_err() {
            BridgeError::Resolve(m) => assert!(
                m.contains("escapes jail"),
                "error must mention 'escapes jail', got: {m}"
            ),
            other => panic!("expected BridgeError::Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn resolve_jailed_allows_relative_path_within_jail() {
        // A plain relative path (no traversal) must resolve Ok under jail_root.
        let ctx = headless_ctx(); // jail_root = PathBuf::from(".")
        let result = resolve_jailed(&ctx, "some/relative/ledger.json");
        assert!(
            result.is_ok(),
            "relative path within jail must resolve Ok, got: {result:?}"
        );
        let p = result.unwrap();
        // jail_path returns an absolute, canonicalized path; verify it is a
        // sub-path of the canonicalized jail root (i.e. still inside the jail).
        let jail_abs = std::path::PathBuf::from(".").canonicalize().unwrap();
        assert!(
            p.starts_with(&jail_abs),
            "resolved path {p:?} should be inside jail root {jail_abs:?}"
        );
    }
}
