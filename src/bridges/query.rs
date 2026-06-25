//! `@query` Router bridge → routes shell commands outbound to the lean-ctx
//! server via `ctx_shell`. Consumer-gated: only runs with `@lean-md shell=allow`
//! (Spec §7). The server is authoritative for allowlist/validation/redaction
//! (Spec §6) — this bridge enforces only the local consumer gate and empty-cmd
//! check.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;
use crate::header::ShellMode;

pub struct QueryBridge;

impl DirectiveBridge for QueryBridge {
    fn name(&self) -> &'static str {
        "query"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        // §7 consumer gate: shell only with `@lean-md shell=allow`.
        if ctx.header.shell != ShellMode::Allow {
            return Err(BridgeError::ShellDenied);
        }
        let cmd = args.raw();
        if cmd.is_empty() {
            return Err(BridgeError::MissingArg("command"));
        }
        // Route outbound — server enforces allowlist/validation/redaction (§6).
        // arg key `command` per appendix-mcp-tools.md §1 ctx_shell row.
        let out = ctx
            .backend
            .call("ctx_shell", serde_json::json!({ "command": cmd }))
            .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}"));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;
    use crate::header::{LeanMdHeader, ShellMode};
    use std::path::PathBuf;

    fn ctx_with_shell(mode: ShellMode) -> Rc<EngineContext> {
        let header = LeanMdHeader {
            shell: mode,
            ..Default::default()
        };
        Rc::new(EngineContext::new(header, PathBuf::from(".")))
    }

    #[test]
    fn denied_without_shell_allow() {
        let err = QueryBridge
            .execute(
                &ctx_with_shell(ShellMode::Deny),
                &DirectiveArgs::parse("git --version"),
            )
            .unwrap_err();
        assert!(matches!(err, BridgeError::ShellDenied));
    }

    #[test]
    #[ignore = "server-enforced; needs lean-ctx in PATH (Task 6.12 integration gate)"]
    fn runs_allowlisted_command_with_shell_allow() {
        // Allowlist/validation/redaction are now enforced server-side (§6).
        // This test verifies the outbound path returns real output from ctx_shell.
        // Requires a live lean-ctx server — skipped in unit CI until Task 6.12.
        let out = QueryBridge
            .execute(
                &ctx_with_shell(ShellMode::Allow),
                &DirectiveArgs::parse("git --version"),
            )
            .unwrap();
        assert!(out.contains("git version"), "got: {out}");
    }

    #[test]
    #[ignore = "server-enforced; needs lean-ctx in PATH (Task 6.12 integration gate)"]
    fn inherits_allowlist_deny_by_default() {
        // Allowlist deny-by-default is now enforced by the lean-ctx server (§6).
        // This test verifies the server rejects non-allowlisted commands via the
        // outbound path. Requires a live lean-ctx server — skipped in unit CI.
        let out = QueryBridge
            .execute(
                &ctx_with_shell(ShellMode::Allow),
                &DirectiveArgs::parse("ls -la"),
            )
            .unwrap();
        // Server returns an error envelope, not a BridgeError, for denied cmds.
        assert!(out.contains("ERROR:") || out.contains("denied") || out.contains("not in"),
            "expected denial envelope, got: {out}");
    }

    #[test]
    fn empty_command_errors() {
        let err = QueryBridge
            .execute(&ctx_with_shell(ShellMode::Allow), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn query_is_registered() {
        assert!(super::super::default_registry().get("query").is_some());
    }
}
