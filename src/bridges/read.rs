//! `@read` Router bridge → the same core as the `ctx_read` MCP tool.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@read <path> [mode=<mode>]` — defaults to `auto`. Phase 1 passes the path
/// through unchanged (jailing `@read` is a §7/Phase-7 concern).
pub struct ReadBridge;

impl DirectiveBridge for ReadBridge {
    fn name(&self) -> &'static str {
        "read"
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let path = args
            .positional(0)
            .or_else(|| args.get("path"))
            .ok_or(BridgeError::MissingArg("path"))?;
        let mode = args.get("mode").unwrap_or("auto");
        match ctx.backend.call(
            "ctx_read",
            serde_json::json!({ "path": path, "mode": mode }),
        ) {
            Ok(out) => Ok(out),
            // A session-less `lean-ctx call ctx_read` fails here (design 2026-08-31
            // §1.1). Degrade instead of losing the anchor — the enclosing @phase and
            // its @on complete sinks must survive an infrastructure outage.
            Err(_) => Ok(read_fallback(ctx, path, mode)),
        }
    }
}

/// Session-free substitute for a failed `ctx_read` (design 2026-08-31 §2.2 B2).
/// `ctx_outline` needs no session and answers the orientation modes with
/// line-numbered signatures; every other mode has no session-free equivalent, so
/// the anchor renders as a visible self-read order rather than a silent comment.
/// The substitute `ctx_outline` call mirrors `OutlineBridge`'s path resolution
/// (`pathx::resolve_tool_path` against `ctx.jail_root`) so a project-relative
/// `@read` path resolves the same way `@outline` would; if resolution itself
/// fails, the self-read order is the correct answer (no panic, no local read).
/// Byte-stable (#498): a pure function of (path, mode, ctx.jail_root).
fn read_fallback(ctx: &Rc<EngineContext>, path: &str, mode: &str) -> String {
    if matches!(mode, "signatures" | "map" | "auto") {
        let root = ctx.jail_root.to_str().unwrap_or(".");
        if let Ok(abs) = crate::pathx::resolve_tool_path(Some(root), None, path)
            && let Ok(out) = ctx
                .backend
                .call("ctx_outline", serde_json::json!({ "path": abs }))
        {
            return format!(
                "<!-- lmd:@read fallback=ctx_outline (ctx_read needs a session) -->\n{out}"
            );
        }
    }
    // The `<!-- lmd:@` prefix matches the same degraded-note marker
    // `auto_findings::extract` already skips (see its guard) — without it, `@on
    // complete capture=auto` mistook the "> ⚠" blockquote for real `ctx_read`
    // output and emitted a garbage session finding.
    format!(
        "<!-- lmd:@read unavailable: no session-free substitute for mode={mode} -->\n\
         > \u{26A0} @read {path} mode={mode} \u{2014} backend has no session.\n\
         >   Read it yourself: ctx_read(path=\"{path}\", mode=\"{mode}\")\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;
    use crate::header::LeanMdHeader;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    #[test]
    fn read_dispatches_to_backend() {
        // Post-B2 contract: a successful backend exit-0 yields Ok (live content or
        // a tool-owned envelope); a real backend failure (lean-ctx absent /
        // jail-refused / session-less) is caught inside `execute` and degrades to
        // `Ok(read_fallback(..))` -- `@read` never surfaces `BridgeError::Backend`
        // to `render::dispatch_result` (unlike a writing bridge, where I2 still
        // aborts the enclosing @phase). The bridge must never panic.
        let f = std::env::temp_dir().join("lmd_read_bridge.txt");
        std::fs::write(&f, "SENTINEL_LINE_42\n").unwrap();
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let args = DirectiveArgs::parse(f.to_str().unwrap());
        match ReadBridge.execute(&ctx, &args) {
            Ok(_) => {}
            Err(BridgeError::Backend(_)) => {}
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
    #[test]
    fn missing_path_errors() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let err = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    struct FailingRead {
        calls: std::rc::Rc<std::cell::RefCell<Vec<(String, serde_json::Value)>>>,
    }
    impl crate::backend::CodeIntelBackend for FailingRead {
        fn call(
            &self,
            tool: &str,
            args: serde_json::Value,
        ) -> Result<String, crate::backend::BackendError> {
            self.calls
                .borrow_mut()
                .push((tool.to_string(), args.clone()));
            match tool {
                "ctx_outline" => Ok("OUTLINE_OK\n".to_string()),
                _ => Err(crate::backend::BackendError::NonZero {
                    code: 2,
                    stderr: "error: -32603: session not available".into(),
                }),
            }
        }
    }

    /// Fixed absolute root — production `jail_root` is always absolute, unlike
    /// the `"."` the other tests in this file use — so the fallback's
    /// `pathx::resolve_tool_path` call is exercised the same way `OutlineBridge`
    /// exercises it (plan-review correction, design 2026-08-31 §2.2 B2).
    fn fallback_root() -> PathBuf {
        std::env::temp_dir().join("lmd_read_fallback_root")
    }

    fn failing_ctx(
        calls: &std::rc::Rc<std::cell::RefCell<Vec<(String, serde_json::Value)>>>,
    ) -> Rc<EngineContext> {
        Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            fallback_root(),
            Box::new(FailingRead {
                calls: calls.clone(),
            }),
        ))
    }

    #[test]
    fn signatures_mode_falls_back_to_outline_exactly_once() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/lib.rs mode=signatures"))
            .unwrap();
        assert!(out.contains("fallback=ctx_outline"), "{out}");
        assert!(out.contains("OUTLINE_OK"), "{out}");
        assert_eq!(
            calls
                .borrow()
                .iter()
                .filter(|(t, _)| t == "ctx_outline")
                .count(),
            1,
            "exactly one substitute call: {:?}",
            calls.borrow()
        );
    }

    #[test]
    fn the_outline_fallback_resolves_the_path_like_outline_bridge_does() {
        // Plan-review correction: OutlineBridge resolves via
        // `pathx::resolve_tool_path` against `ctx.jail_root` before calling the
        // backend, so a project-relative `@read` path could otherwise resolve
        // differently (or not at all) against the real CLI backend.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/lib.rs mode=signatures"))
            .unwrap();
        let sent = calls
            .borrow()
            .iter()
            .find(|(t, _)| t == "ctx_outline")
            .expect("ctx_outline must have been called")
            .1
            .clone();
        let sent_path = sent["path"].as_str().unwrap().to_string();
        let expected = fallback_root()
            .join("src/lib.rs")
            .to_string_lossy()
            .into_owned();
        assert_eq!(sent_path, expected);
        assert!(Path::new(&sent_path).is_absolute(), "{sent_path}");
    }

    #[test]
    fn full_mode_renders_a_self_read_order_without_a_second_call() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/seal.rs mode=full"))
            .unwrap();
        assert!(out.contains("src/seal.rs"), "{out}");
        assert!(
            out.contains("mode=\"full\""),
            "the order must be copy-pasteable: {out}"
        );
        assert!(
            !calls.borrow().iter().any(|(t, _)| t == "ctx_outline"),
            "no substitute exists for full: {:?}",
            calls.borrow()
        );
    }

    #[test]
    fn the_read_fallback_is_byte_stable() {
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let args = DirectiveArgs::parse("src/lib.rs mode=signatures");
        let a = ReadBridge.execute(&ctx, &args).unwrap();
        let b = ReadBridge.execute(&ctx, &args).unwrap();
        assert_eq!(a, b, "#498: two renders of the same source are identical");
    }
}
