//! `@read` Router bridge → the same core as the `ctx_read` MCP tool.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::backend::BackendError;
use crate::engine::EngineContext;
use crate::render::{LMD_NOTE_PREFIX, sanitize_comment};

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
            // its @on complete sinks must survive an infrastructure outage. The real
            // cause (`e`) is threaded into the fallback note (C2 review correction):
            // discarding it made every failure render the same "backend has no
            // session" wording even when the actual cause was e.g. a missing
            // `lean-ctx` binary (`BackendError::Spawn`) or a server-side jail
            // rejection (`BackendError::NonZero`) — both factually wrong claims.
            Err(e) => Ok(read_fallback(ctx, path, mode, &e)),
        }
    }
}

/// Session-free substitute for a failed `ctx_read` (design 2026-08-31 §2.2 B2).
/// `ctx_outline` needs no session and answers the orientation modes with
/// line-numbered signatures; every other mode has no session-free equivalent, so
/// the anchor renders as a visible self-read order rather than a silent comment.
/// The substitute `ctx_outline` call mirrors `OutlineBridge`'s path resolution
/// (`pathx::resolve_tool_path` against `ctx.jail_root`) so a project-relative
/// `@read` path resolves the same way `@outline` would. `resolve_tool_path` is
/// currently infallible (all three of its branches return `Ok`; the `Result` in
/// its signature exists only for contract parity with `core::path_resolve`), so
/// the `if let Ok(abs) = ...` guard below never actually takes its failure arm
/// today — it is written this way (matching `OutlineBridge`) for defensive
/// future-proofing, not because failure is a live case today (C4 review
/// correction: the previous wording described this dead arm as a plausible
/// outcome). Note also: `OutlineBridge` itself treats a resolve failure as
/// `BridgeError::Resolve` (fatal — an author error per the global constraints),
/// whereas this fallback would silently degrade it instead; that divergence is
/// inert today for the same reason (the arm is unreachable), but would need
/// reconciling if `resolve_tool_path` ever grows a real failure path.
/// `path`/`mode` are author-controlled (`DirectiveArgs` quoting allows literal
/// newlines via `\n`-escapes) and get interpolated into both an HTML comment
/// and a `>`-blockquote below; `cause` is the backend's own error text. All
/// three are sanitized (C1 review correction) the same way
/// `render::degraded_note` sanitizes its fields — a comment-delimiter
/// (`-->`/`<!--`) or raw-newline breakout is otherwise possible from any of
/// them.
/// Byte-stable (#498): a pure function of (path, mode, cause, ctx.jail_root).
fn read_fallback(ctx: &Rc<EngineContext>, path: &str, mode: &str, cause: &BackendError) -> String {
    let safe_cause = sanitize_comment(&cause.to_string());
    if matches!(mode, "signatures" | "map" | "auto") {
        let root = ctx.jail_root.to_str().unwrap_or(".");
        if let Ok(abs) = crate::pathx::resolve_tool_path(Some(root), None, path)
            && let Ok(out) = ctx
                .backend
                .call("ctx_outline", serde_json::json!({ "path": abs }))
        {
            return format!(
                "{LMD_NOTE_PREFIX}read fallback=ctx_outline (ctx_read failed: {safe_cause}) -->\n{out}"
            );
        }
    }
    // The `<!-- lmd:@` prefix (`LMD_NOTE_PREFIX`) matches the same
    // degraded-note marker `auto_findings::extract` already skips (see its
    // guard) — without it, `@on complete capture=auto` mistook the "> ⚠"
    // blockquote for real `ctx_read` output and emitted a garbage session
    // finding.
    let safe_path = sanitize_comment(path);
    let safe_mode = sanitize_comment(mode);
    // A cause that looks like a deliberate server-side path rejection (jail /
    // deny) must not be followed by an order to read the same path some other
    // way — that would just ask the agent to route around a refusal. Any other
    // cause (infra outage: lean-ctx missing, a transient session hiccup, …) is
    // not a policy decision, so the self-read order still stands (C2 review
    // decision: suppress rather than reword — a refused path has no safe
    // rephrasing of "read it yourself").
    if looks_like_a_deliberate_refusal(cause) {
        return format!(
            "{LMD_NOTE_PREFIX}read unavailable: no session-free substitute for mode={safe_mode}; cause: {safe_cause} -->\n\
             > \u{26A0} @read {safe_path} mode={safe_mode} \u{2014} ctx_read refused this path: {safe_cause}\n\
             >   This path was rejected server-side; reading it another way is not a fix.\n"
        );
    }
    format!(
        "{LMD_NOTE_PREFIX}read unavailable: no session-free substitute for mode={safe_mode}; cause: {safe_cause} -->\n\
         > \u{26A0} @read {safe_path} mode={safe_mode} \u{2014} ctx_read failed: {safe_cause}\n\
         >   Read it yourself: ctx_read(path=\"{safe_path}\", mode=\"{safe_mode}\")\n"
    )
}

/// Best-effort classification of a `BackendError` as a deliberate server-side
/// refusal (PathJail / access denial) rather than an infrastructure outage.
/// `Spawn`/`Io` are always infra (lean-ctx itself never ran); a `NonZero` exit
/// is infra unless its stderr names a jail/deny reason. Heuristic, not a
/// contract: worst case it under-detects and the self-read order still shows
/// (never a false "safe to reroute" claim beyond that).
fn looks_like_a_deliberate_refusal(e: &BackendError) -> bool {
    match e {
        BackendError::NonZero { stderr, .. } => {
            let s = stderr.to_ascii_lowercase();
            s.contains("jail")
                || s.contains("denied")
                || s.contains("escapes")
                || s.contains("outside")
        }
        _ => false,
    }
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

    #[test]
    fn auto_mode_falls_back_to_outline_like_signatures_does() {
        // C5 coverage gap: `auto` is the *default* mode (the most common case
        // in practice) but was untested — a `matches!` typo dropping "auto"
        // from the outline-fallback arm would have gone unnoticed.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/lib.rs"))
            .unwrap();
        assert!(out.contains("fallback=ctx_outline"), "{out}");
        assert!(out.contains("OUTLINE_OK"), "{out}");
    }

    #[test]
    fn map_mode_falls_back_to_outline_like_signatures_does() {
        // C5 coverage gap: `map` was untested alongside `signatures`.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/lib.rs mode=map"))
            .unwrap();
        assert!(out.contains("fallback=ctx_outline"), "{out}");
        assert!(out.contains("OUTLINE_OK"), "{out}");
    }

    #[test]
    fn both_fallback_branches_start_with_the_shared_lmd_note_prefix() {
        // C5 coverage gap: only a `phases.rs` e2e test (far from the producer)
        // pinned the `<!-- lmd:@` prefix, and only for the non-outline branch.
        // Pin it here, at the producer, for both branches, via the shared
        // `render::LMD_NOTE_PREFIX` constant `auto_findings::extract` guards on.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        let outline_branch = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/lib.rs mode=signatures"))
            .unwrap();
        assert!(
            outline_branch.starts_with(crate::render::LMD_NOTE_PREFIX),
            "{outline_branch}"
        );
        let self_read_branch = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/seal.rs mode=full"))
            .unwrap();
        assert!(
            self_read_branch.starts_with(crate::render::LMD_NOTE_PREFIX),
            "{self_read_branch}"
        );
    }

    #[test]
    fn read_fallback_sanitizes_breakout_payloads_in_both_fields() {
        // C1 review correction: `path`/`mode` are author-controlled
        // (`DirectiveArgs` quoting allows literal newlines via `\n`-escapes)
        // and get interpolated into both an HTML comment and a `>`-blockquote.
        // Mirrors the Task-1 precedent
        // (`render::degraded_note_sanitizes_both_fields_and_is_byte_stable`).
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = failing_ctx(&calls);
        // Decoded by `DirectiveArgs`' double-quote `\n`-escape into a literal
        // embedded newline in `path`; `mode` carries its own comment-delimiter
        // breakout.
        let args = DirectiveArgs::parse(r#""a-->b<!--c\nd.rs" mode="x-->y<!--z""#);
        let out = ReadBridge.execute(&ctx, &args).unwrap();
        assert!(
            out.contains("a--&gt;b&lt;!--c\\nd.rs"),
            "path must be sanitized as a single escaped unit, incl. the \
             embedded newline: {out}"
        );
        assert!(
            out.contains("x--&gt;y&lt;!--z"),
            "mode must be sanitized: {out}"
        );
        for line in out.lines() {
            assert!(
                line.starts_with("<!--") || line.starts_with('>'),
                "a line broke out of the comment/blockquote wrapper: {line:?} in {out}"
            );
        }
    }

    #[test]
    fn a_non_session_backend_failure_shows_its_real_cause() {
        // C2 review correction: every backend failure used to render the same
        // hardcoded "backend has no session" text regardless of cause. A
        // `BackendError::Spawn` (lean-ctx missing from PATH) has nothing to do
        // with sessions — the note must say so, not assert a cause it never saw.
        struct SpawnFailure;
        impl crate::backend::CodeIntelBackend for SpawnFailure {
            fn call(
                &self,
                _tool: &str,
                _args: serde_json::Value,
            ) -> Result<String, crate::backend::BackendError> {
                Err(crate::backend::BackendError::Spawn(
                    "lean-ctx: command not found".into(),
                ))
            }
        }
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            fallback_root(),
            Box::new(SpawnFailure),
        ));
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/seal.rs mode=full"))
            .unwrap();
        assert!(out.contains("lean-ctx: command not found"), "{out}");
        assert!(
            !out.contains("backend has no session"),
            "must not assert a cause it does not know: {out}"
        );
    }

    #[test]
    fn a_deliberate_jail_rejection_does_not_suggest_reading_the_path_another_way() {
        // C2 review: the reviewer's empirical case — `@read /etc/passwd`
        // rendered "backend has no session" plus an order to read the
        // deliberately refused path anyway. A recognizable server-side
        // jail/deny cause must drop that order instead of inviting the agent
        // to route around the refusal.
        struct JailReject;
        impl crate::backend::CodeIntelBackend for JailReject {
            fn call(
                &self,
                _tool: &str,
                _args: serde_json::Value,
            ) -> Result<String, crate::backend::BackendError> {
                Err(crate::backend::BackendError::NonZero {
                    code: 2,
                    stderr: "error: -32602: path escapes jail: /etc/passwd".into(),
                })
            }
        }
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            fallback_root(),
            Box::new(JailReject),
        ));
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("/etc/passwd mode=full"))
            .unwrap();
        assert!(out.contains("escapes jail"), "{out}");
        assert!(
            !out.contains("Read it yourself"),
            "must not suggest routing around a refusal: {out}"
        );
    }
}
