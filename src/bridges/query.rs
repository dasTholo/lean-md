//! `@query` Router bridge → executes a shell command via the same path as the
//! `ctx_shell` MCP tool, then compresses output. Consumer-gated: only runs with
//! `@lean-md shell=allow` (Spec §7). Inherits validate_command (strict-mode
//! $()/backtick block) + shell_allowlist + secret redaction — invents no new
//! deny-patterns.
use std::collections::HashMap;
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
        // Inherited lean-ctx shell defenses — no new deny-patterns (§7).
        if let Some(rejection) = crate::tools::ctx_shell::validate_command(cmd) {
            return Err(BridgeError::ShellRejected(rejection));
        }
        if let Err(msg) = crate::core::shell_allowlist::check_shell_allowlist(cmd) {
            return Err(BridgeError::ShellRejected(msg));
        }
        let cwd = ctx.jail_root.to_string_lossy().to_string();
        let (raw_output, exit) =
            crate::server::execute::execute_command_with_env(cmd, &cwd, &HashMap::new());
        // Secret-Redaction (only if enabled) — inherited, not reinvented (§7).
        let cfg = crate::core::config::Config::load();
        let safe_output = if cfg.secret_detection.enabled {
            crate::core::secret_detection::scan_and_redact(&raw_output, &cfg.secret_detection).0
        } else {
            raw_output
        };
        let compressed =
            crate::tools::ctx_shell::handle(cmd, &safe_output, exit, crate::crp_proto::CrpMode::Off);
        Ok(crate::core::redaction::redact_text_if_enabled(&compressed))
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
    fn runs_allowlisted_command_with_shell_allow() {
        // Hermetic: pin the allowlist via override so the test does NOT depend on
        // the user's config (default ships a NON-empty deny-by-default allowlist).
        // `git` is representative of @query's real purpose — a shell-only tool with
        // no native lmd directive. nextest = process-per-test, so the env override is isolated.
        crate::test_env::set_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE", "git");
        let out = QueryBridge
            .execute(
                &ctx_with_shell(ShellMode::Allow),
                &DirectiveArgs::parse("git --version"),
            )
            .unwrap();
        crate::test_env::remove_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE");
        assert!(out.contains("git version"), "got: {out}");
    }

    #[test]
    fn inherits_allowlist_deny_by_default() {
        // §7: a non-allowlisted base command is hard-blocked — @query inherits the
        // deny-by-default gate via check_shell_allowlist, it does not reinvent it.
        crate::test_env::set_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE", "git");
        let err = QueryBridge
            .execute(
                &ctx_with_shell(ShellMode::Allow),
                &DirectiveArgs::parse("ls -la"),
            )
            .unwrap_err();
        crate::test_env::remove_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE");
        assert!(matches!(err, BridgeError::ShellRejected(_)));
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
