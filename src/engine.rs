//! Engine wiring: header pre-scan → build the rushdown render closure with the
//! lmd parser + renderer extensions → render the body. `render_body` is the
//! re-entry point used by `@include` for recursive fragment rendering.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::core::cache::SessionCache;
use crate::core::call_graph::CallGraph;
use crate::core::graph_index::{self, ProjectIndex};
use rushdown::new_markdown_to_html;

use super::bridges::{default_registry, BridgeError, BridgeRegistry};
use super::fragments::FragmentRegistry;
use super::header::{parse_header, LeanMdHeader};
use super::parser::lmd_parser_extension;
use super::render::lmd_renderer_extension;

/// Per-render engine state shared (via `Rc`) with the renderer hook and bridges.
pub struct EngineContext {
    pub header: LeanMdHeader,
    pub jail_root: PathBuf,
    pub fragments: FragmentRegistry,
    pub registry: BridgeRegistry,
    /// ONE session cache shared by every `@read` in this render — warm across
    /// re-reads so the 2nd read of a path is a ~13-tok cache-hit / auto-delta,
    /// never a full re-dump, WITHOUT `fresh`/`raw` (spec §4.2a Read→Delta).
    pub cache: RefCell<SessionCache>,
    pub max_chain_depth: usize,
    depth: Cell<usize>,
    /// Lazy per-render memo of the static graph index (one build, shared by
    /// every `@graph` op in this render — §4.2a-analog).
    graph_index: RefCell<Option<Rc<ProjectIndex>>>,
    /// Lazy per-render memo of the call graph (built from `graph_index`).
    call_graph: RefCell<Option<Rc<CallGraph>>>,
}

impl EngineContext {
    pub fn new(header: LeanMdHeader, jail_root: PathBuf) -> Self {
        Self {
            header,
            jail_root,
            fragments: FragmentRegistry::with_builtins(),
            registry: default_registry(),
            cache: RefCell::new(SessionCache::new()),
            max_chain_depth: 16,
            depth: Cell::new(0),
            graph_index: RefCell::new(None),
            call_graph: RefCell::new(None),
        }
    }
    /// Increment the include-chain depth; error past `max_chain_depth` (§7).
    pub fn enter(&self) -> Result<(), BridgeError> {
        let d = self.depth.get();
        if d >= self.max_chain_depth {
            return Err(BridgeError::DepthExceeded);
        }
        self.depth.set(d + 1);
        Ok(())
    }
    pub fn leave(&self) {
        self.depth.set(self.depth.get().saturating_sub(1));
    }

    /// Lazy-build + memoize the static project index for this render.
    pub fn index(&self) -> Rc<ProjectIndex> {
        if let Some(existing) = self.graph_index.borrow().as_ref() {
            return existing.clone();
        }
        let root = self.jail_root.to_str().unwrap_or(".");
        let built = Rc::new(graph_index::load_or_build(root));
        *self.graph_index.borrow_mut() = Some(built.clone());
        built
    }

    /// Lazy-build + memoize the call graph (depends on `index()`).
    pub fn call_graph(&self) -> Rc<CallGraph> {
        if let Some(existing) = self.call_graph.borrow().as_ref() {
            return existing.clone();
        }
        let index = self.index();
        let root = self.jail_root.to_str().unwrap_or(".");
        let built = Rc::new(CallGraph::load_or_build(root, &index));
        let _ = built.save();
        *self.call_graph.borrow_mut() = Some(built.clone());
        built
    }
}

/// Top-level entry: pre-scan the `@lean-md` header, then render the body.
pub fn render(input: &str) -> String {
    let (header, body) = parse_header(input);
    let ctx = Rc::new(EngineContext::new(header, PathBuf::from(".")));
    render_body(&ctx, body)
}

/// Build a fresh rushdown render closure wired with the lmd extensions and
/// render `body`. Re-entrant: `@include` calls this for fragment content.
pub fn render_body(ctx: &Rc<EngineContext>, body: &str) -> String {
    let render = new_markdown_to_html(
        rushdown::parser::Options::default(),
        rushdown::renderer::html::Options::default(),
        lmd_parser_extension(),
        lmd_renderer_extension(ctx.clone()),
    );
    let mut out = String::new();
    let _ = render(&mut out, body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_include_builtin() {
        let out = render("@include hard-rules\n");
        assert!(out.contains("lean-ctx"), "got: {out}");
    }
    #[test]
    fn renders_read_directive() {
        let f = std::env::temp_dir().join("lmd_engine_read.txt");
        std::fs::write(&f, "ENGINE_SENTINEL_7\n").unwrap();
        let out = render(&format!("@read {}\n", f.to_str().unwrap()));
        assert!(out.contains("ENGINE_SENTINEL_7"), "got: {out}");
    }
    // Read→Delta guarantee (spec §4.2a / §6 gate). The `[unchanged]` cache-hit
    // stub is a `mode=full` feature: `auto` deliberately compresses (and auto
    // re-reads are already compact), so the clean single-sentinel stub only lands
    // in full mode — verified empirically 2026-06-02. The fixture is multi-line
    // with the sentinel on line 2 so the cache-hit proof-line (first file line)
    // never leaks the sentinel into a stub. Three reads match the spec's
    // "3-Read/mode=full" gate.
    #[test]
    fn reread_same_path_is_cache_hit_not_full() {
        let f = std::env::temp_dir().join("lmd_reread_cache.txt");
        std::fs::write(&f, "// reread fixture header\nREREAD_SENTINEL_99\n").unwrap();
        let p = f.to_str().unwrap();
        let out = render(&format!(
            "@read {p} mode=full\n\n@read {p} mode=full\n\n@read {p} mode=full\n"
        ));
        let sentinels = out.matches("REREAD_SENTINEL_99").count();
        let stubs = out.matches("[unchanged").count();
        assert_eq!(
            sentinels, 1,
            "only the first full read carries the sentinel; re-reads must be cache-hit stubs; got {sentinels}x in: {out}"
        );
        assert!(
            stubs >= 2,
            "the 2nd and 3rd reads must be [unchanged] cache-hit stubs; got {stubs} in: {out}"
        );
    }

    #[test]
    fn inline_comment_injection_is_inert() {
        // F-2 e2e: `{{ -->x }}` must NOT be claimed as a directive (invalid name
        // charset) and must NOT inject an HTML comment into the render.
        let out = render("pre {{ -->x }} post\n");
        assert!(
            !out.contains("<!-- lmd"),
            "injection must not reach the comment fallback; got: {out}"
        );
        assert!(out.contains("pre") && out.contains("post"), "got: {out}");
    }
    #[test]
    fn header_is_stripped_from_output() {
        let out = render("@lean-md 0.1\nconsumer: ai\n\n@include hard-rules\n");
        assert!(
            !out.contains("@lean-md"),
            "header must not appear; got: {out}"
        );
        assert!(out.contains("lean-ctx"));
    }
    #[test]
    fn unknown_directive_renders_comment() {
        let out = render("@frobnicate x\n");
        assert!(out.contains("unknown directive @frobnicate"), "got: {out}");
    }
    #[test]
    fn inline_include_dispatches() {
        let out = render("rules: {{ include hard-rules }}\n");
        assert!(
            out.contains("lean-ctx"),
            "inline dispatch must fire; got: {out}"
        );
    }

    #[test]
    fn query_denied_without_shell_allow() {
        // Default header => shell=deny => @query must not execute.
        let out = render("@query git --version\n");
        assert!(
            !out.contains("git version"),
            "shell must be denied without shell=allow; got: {out}"
        );
    }

    #[test]
    fn query_runs_with_shell_allow_header() {
        // Hermetic allowlist pin (see bridge unit test). nextest = process-per-test.
        std::env::set_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE", "git");
        let out = render("@lean-md\nshell: allow\n\n@query git --version\n");
        std::env::remove_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE");
        assert!(out.contains("git version"), "got: {out}");
    }

    #[test]
    fn index_memo_returns_same_handle_twice() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let a = ctx.index();
        let b = ctx.index();
        assert!(
            Rc::ptr_eq(&a, &b),
            "index() must memoize one build per render"
        );
    }

    #[test]
    fn graph_directive_renders_dependents_e2e() {
        // Render a `@graph dependents` directive end-to-end against this repo.
        let out = render("@graph dependents rust/src/lmd/engine.rs\n");
        // Either a dependents list or the graceful "No dependents" line — the
        // directive must be dispatched (not the unknown-directive fallback).
        assert!(
            out.contains("dependent") || out.contains("No dependents"),
            "got: {out}"
        );
        assert!(!out.contains("unknown directive"), "got: {out}");
    }

    #[test]
    fn edit_invalidates_reader_set_read_and_graph() {
        // Phase-3.1 gate: after @edit, the @read+@graph reader set must observe
        // the post-edit state — not a stale warm-cache hit.
        //
        // `render()` uses jail_root="." (project root), which blocks /tmp writes.
        // Use `render_body` with an explicit EngineContext whose jail_root is the
        // temp dir, matching the pattern in bridges/edit.rs unit tests.
        let dir = std::env::temp_dir().join("lmd_gate_edit_reader_set");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("fixture.txt");
        std::fs::write(&f, "// gate fixture\nGATE_BEFORE_42\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let doc = format!(
            "@read {p} mode=full\n\n\
             @edit {p} old=\"GATE_BEFORE_42\" new=\"GATE_AFTER_42\"\n\n\
             @read {p} mode=full\n\n\
             @graph dependents {p}\n"
        );
        let out = render_body(&ctx, &doc);

        // The post-edit read must carry the NEW token.
        assert!(
            out.contains("GATE_AFTER_42"),
            "post-edit read missing new bytes: {out}"
        );
        // GATE_BEFORE_42 may appear at most twice: once in the first (pre-edit)
        // @read and once in the @edit evidence diff ("-GATE_BEFORE_42").  A third
        // occurrence would mean the second @read served a stale cache hit.
        assert!(
            out.matches("GATE_BEFORE_42").count() <= 2,
            "post-edit @read must not serve stale cache — got a third occurrence of \
             old bytes: {out}"
        );
        // @graph still dispatches (not the unknown-directive fallback) after the edit.
        assert!(
            !out.contains("unknown directive"),
            "graph broke post-edit: {out}"
        );
    }

    #[test]
    fn symbol_overview_renders_e2e() {
        // Phase-3.2 gate: @symbol must dispatch through the full render pipeline,
        // not the unknown-directive fallback.
        let dir = std::env::temp_dir().join("lmd_gate_symbol_overview");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "pub fn rendered_symbol() {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, &format!("@symbol overview {p}\n"));

        assert!(
            !out.contains("unknown directive"),
            "symbol must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
        // I-2: symbols_overview resolves the tree-sitter fallthrough BEFORE the
        // LSP `open_file` round-trip, so a headless run never leaks `open_file`.
        assert!(
            !out.contains("open_file"),
            "headless overview must not leak open_file error: {out}"
        );
    }

    #[test]
    fn symbol_unknown_op_renders_bridge_error_not_unknown_directive() {
        // The directive IS known; only the op is wrong → bridge error comment,
        // never the unknown-directive fallback.
        let out = render("@symbol frobnicate x.rs\n");
        assert!(!out.contains("unknown directive @symbol"), "got: {out}");
        assert!(
            out.contains("unknown @symbol op"),
            "expected op error: {out}"
        );
    }

    // ── Phase 3.3 gate: @refactor e2e through the full render pipeline ──────

    #[test]
    fn refactor_rename_renders_backend_required_e2e() {
        // Phase-3.3 gate (positive): @refactor rename must dispatch through the
        // full render pipeline and return the BACKEND_REQUIRED envelope verbatim.
        // Headless has no IDE → backend degrades cleanly; the directive must NOT
        // fall through to the unknown-directive comment.
        let dir = std::env::temp_dir().join("lmd_gate_refactor_rename");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "pub fn my_func() {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(
            &ctx,
            &format!("@refactor rename path={p} line=1 new=renamed_func\n"),
        );

        assert!(
            !out.contains("unknown directive"),
            "@refactor must dispatch (not unknown-directive fallback): {out}"
        );
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "headless must degrade to BACKEND_REQUIRED envelope, got: {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
    }

    #[test]
    fn refactor_inline_force_degrades_cleanly_e2e() {
        // Phase-3.3 gate (negative): exercises the full render-path degradation of
        // `@refactor inline force path=… line=… plan_hash=…` in a headless engine.
        //
        // What this test proves (end-to-end render path):
        //   • The directive is dispatched through the bridge (no unknown-directive
        //     comment leaks into the output).
        //   • The inline+force+plan_hash combination produces a clean, non-panicking
        //     envelope (BACKEND_REQUIRED or ERROR/UNSUPPORTED) over the *full* render
        //     pipeline — not just at bridge/unit level.
        //   • The output is non-empty (no silent swallow).
        //
        // What this test does NOT prove (already covered at unit level):
        //   • The `force` flag is NOT forwarded in the inline op-map — that Map
        //     invariant is proven by `inline_apply_force_does_not_set_force_key`
        //     in refactor.rs. A BACKEND_REQUIRED envelope never echoes caller args,
        //     so checking for `"force": true` absence here would be trivially true
        //     and prove nothing about the bridge.
        let dir = std::env::temp_dir().join("lmd_gate_refactor_inline_force");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "pub fn inlineable() {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(
            &ctx,
            &format!("@refactor inline force path={p} line=1 plan_hash=deadbeef\n"),
        );

        // 1. Bridge dispatched — no unknown-directive fallback leaked.
        assert!(
            !out.contains("unknown directive"),
            "@refactor inline must dispatch (not unknown-directive fallback): {out}"
        );
        // 2. No silent swallow — output must not be empty.
        assert!(!out.trim().is_empty(), "render produced empty output");
        // 3. Clean headless-degradation envelope over the full render path.
        assert!(
            out.contains("BACKEND_REQUIRED")
                || out.starts_with("ERROR")
                || out.contains("UNSUPPORTED"),
            "inline+force must degrade to clean envelope, got: {out}"
        );
    }
}
