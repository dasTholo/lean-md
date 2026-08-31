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
            // The cause travels as a `BridgeError`, not the bare
            // `BackendError`: its `Display` carries the `BACKEND_REQUIRED:`
            // marker, which `render::degraded_note` — the other producer of
            // this failure class — also emits. One wording, one source (M-3).
            Err(e) => Ok(read_fallback(ctx, path, mode, &BridgeError::Backend(e))),
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
/// and a `>`-blockquote below; `cause` is the `BridgeError` display (the
/// backend's own error text behind the shared `BACKEND_REQUIRED:` marker). All
/// three are sanitized (C1 review correction) the same way
/// `render::degraded_note` sanitizes its fields — a comment-delimiter
/// (`-->`/`<!--`) or raw-newline breakout is otherwise possible from any of
/// them.
/// Byte-stable (#498): a pure function of (path, mode, cause, ctx.jail_root).
fn read_fallback(ctx: &Rc<EngineContext>, path: &str, mode: &str, cause: &BridgeError) -> String {
    let safe_cause = sanitize_comment(&cause.to_string());
    // `path`/`mode` are sanitized once, up front: every branch below
    // interpolates them into an HTML comment and/or a `>`-blockquote.
    let safe_path = sanitize_comment(path);
    let safe_mode = sanitize_comment(mode);

    // A cause that looks like a deliberate server-side path rejection (jail /
    // deny) must not be followed by an order to read the same path some other
    // way — that would just ask the agent to route around a refusal. Any other
    // cause (infra outage: lean-ctx missing, a transient session hiccup, …) is
    // not a policy decision, so the self-read order still stands (C2 review
    // decision: suppress rather than reword — a refused path has no safe
    // rephrasing of "read it yourself").
    //
    // This runs BEFORE the `ctx_outline` substitute below (M-2): that
    // substitute is the same "try the refused path another way" move, only as
    // a call instead of a sentence. The server refuses `ctx_outline` on a
    // jailed path with byte-identical stderr (measured against lean-ctx
    // 3.10.0), so the retry never produced content — it only spent a
    // subprocess per anchor and contradicted the note it was about to render.
    if looks_like_a_deliberate_refusal(cause) {
        return format!(
            "{LMD_NOTE_PREFIX}read unavailable: no session-free substitute for mode={safe_mode}; cause: {safe_cause} -->\n\
             > \u{26A0} @read {safe_path} mode={safe_mode} \u{2014} ctx_read refused this path: {safe_cause}\n\
             >   This path was rejected server-side; reading it another way is not a fix.\n"
        );
    }

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
    format!(
        "{LMD_NOTE_PREFIX}read unavailable: no session-free substitute for mode={safe_mode}; cause: {safe_cause} -->\n\
         > \u{26A0} @read {safe_path} mode={safe_mode} \u{2014} ctx_read failed: {safe_cause}\n\
         >   Read it yourself: ctx_read(path=\"{safe_path}\", mode=\"{safe_mode}\")\n"
    )
}

/// Substrings that only a deliberate server-side path rejection produces.
/// Both are verbatim fragments of lean-ctx's own jail errors, measured against
/// lean-ctx 3.10.0 (`core/error.rs` `path escapes project root: {path} (root:
/// {root})` plus `core/pathjail.rs`'s `Access denied: outside active project
/// (…)` hint); a real refusal carries both, so either one alone is enough.
const REFUSAL_MARKERS: &[&str] = &["path escapes project root", "access denied: outside active"];

/// Best-effort classification of a `BridgeError` as a deliberate server-side
/// refusal (PathJail / access denial) rather than an infrastructure outage.
/// Only a `Backend(NonZero)` can qualify: `Spawn`/`Io` mean lean-ctx itself
/// never ran, and every non-`Backend` variant is an author error that never
/// reaches this function. A `NonZero` exit is infra unless its stderr contains
/// one of the full `REFUSAL_MARKERS` phrases.
///
/// Deliberately biased towards under-detection (M-1): the previous version
/// matched the bare words `jail`/`denied`/`escapes`/`outside`, which arbitrary
/// error prose contains, and a false positive renders "This path was rejected
/// server-side" for a plain outage — a claim about a server decision that was
/// never made. Under-detection costs only a self-read order for a path that
/// will refuse it again; over-detection states something untrue.
fn looks_like_a_deliberate_refusal(e: &BridgeError) -> bool {
    match e {
        BridgeError::Backend(BackendError::NonZero { stderr, .. }) => {
            let s = stderr.to_ascii_lowercase();
            REFUSAL_MARKERS.iter().any(|m| s.contains(m))
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
        // to route around the refusal. The stderr is the real lean-ctx wording
        // (M-1) — the invented "path escapes jail" this test used to carry only
        // ever exercised the over-broad `contains("jail")` arm.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = refusing_ctx(&calls, REAL_JAIL_STDERR);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("/etc/passwd mode=full"))
            .unwrap();
        assert!(out.contains("escapes project root"), "{out}");
        assert!(
            !out.contains("Read it yourself"),
            "must not suggest routing around a refusal: {out}"
        );
    }

    /// The verbatim stderr of a real `lean-ctx call ctx_read` on a jailed path
    /// (measured against lean-ctx 3.10.0), so the refusal heuristic is pinned to
    /// wording lean-ctx actually emits instead of an invented one.
    const REAL_JAIL_STDERR: &str = "error: path resolution failed: path escapes \
         project root: /etc/passwd (root: /srv/p). Access denied: outside active \
         project (/srv/p). To allow: open that project in a new window, or: \
         LEAN_CTX_EXTRA_ROOTS=/etc.";

    /// Backend that refuses `ctx_read` server-side but would happily answer
    /// `ctx_outline` — the only way to see whether the substitute is attempted.
    struct RefusingRead {
        calls: std::rc::Rc<std::cell::RefCell<Vec<(String, serde_json::Value)>>>,
        stderr: &'static str,
    }
    impl crate::backend::CodeIntelBackend for RefusingRead {
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
                    stderr: self.stderr.into(),
                }),
            }
        }
    }

    fn refusing_ctx(
        calls: &std::rc::Rc<std::cell::RefCell<Vec<(String, serde_json::Value)>>>,
        stderr: &'static str,
    ) -> Rc<EngineContext> {
        Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            fallback_root(),
            Box::new(RefusingRead {
                calls: calls.clone(),
                stderr,
            }),
        ))
    }

    #[test]
    fn a_refused_path_is_not_retried_through_the_outline_substitute() {
        // M-2: the refusal check used to run AFTER the `signatures|map|auto`
        // outline arm, so a deliberately refused path was immediately offered to
        // a second tool on the same path. The server refuses `ctx_outline` for
        // the same reason (measured: identical stderr), so the retry could only
        // ever waste a subprocess — while the rendered text claimed the opposite
        // ("reading it another way is not a fix").
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = refusing_ctx(&calls, REAL_JAIL_STDERR);
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("/etc/passwd mode=signatures"))
            .unwrap();
        assert!(
            !calls.borrow().iter().any(|(t, _)| t == "ctx_outline"),
            "a refusal must not be routed around by a second tool: {:?}",
            calls.borrow()
        );
        assert!(
            out.contains("rejected server-side"),
            "the refusal note must win over the outline substitute: {out}"
        );
    }

    #[test]
    fn an_infrastructure_failure_that_merely_says_outside_is_not_a_refusal() {
        // M-1: the heuristic matched the bare substrings "outside"/"denied"/
        // "jail"/"escapes", which any error prose can contain. A false positive
        // renders "This path was rejected server-side" for a plain outage —
        // asserting a server decision that never happened.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = refusing_ctx(
            &calls,
            "error: -32603: the indexer died outside the request window; retry",
        );
        let out = ReadBridge
            .execute(&ctx, &DirectiveArgs::parse("src/seal.rs mode=full"))
            .unwrap();
        assert!(
            out.contains("Read it yourself"),
            "an infra outage is not a policy decision — the self-read order stands: {out}"
        );
        assert!(
            !out.contains("rejected server-side"),
            "must not claim a server-side rejection it never saw: {out}"
        );
    }

    #[test]
    fn every_fallback_branch_keeps_the_backend_required_marker() {
        // M-3: `read_fallback` interpolated `BackendError::to_string()` while
        // `render::degraded_note` interpolates `BridgeError`, which prefixes
        // `BACKEND_REQUIRED:`. Two producers of the same failure class must not
        // drift — the marker is the documented backwards-compatibility anchor.
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let self_read = ReadBridge
            .execute(
                &failing_ctx(&calls),
                &DirectiveArgs::parse("src/seal.rs mode=full"),
            )
            .unwrap();
        assert!(self_read.contains("BACKEND_REQUIRED:"), "{self_read}");

        let outline = ReadBridge
            .execute(
                &failing_ctx(&calls),
                &DirectiveArgs::parse("src/lib.rs mode=signatures"),
            )
            .unwrap();
        assert!(outline.contains("BACKEND_REQUIRED:"), "{outline}");

        let refused = ReadBridge
            .execute(
                &refusing_ctx(&calls, REAL_JAIL_STDERR),
                &DirectiveArgs::parse("/etc/passwd mode=full"),
            )
            .unwrap();
        assert!(refused.contains("BACKEND_REQUIRED:"), "{refused}");
    }
}
