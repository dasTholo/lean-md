//! Engine wiring: header pre-scan → build the rushdown render closure with the
//! lmd parser + renderer extensions → render the body. `render_body` is the
//! re-entry point used by `@include` for recursive fragment rendering.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use crate::core::cache::SessionCache;
use crate::core::call_graph::{CallGraph, CallGraphInputs};
use crate::core::graph_index::{self, ProjectIndex};
use rushdown::new_markdown_to_html;

use super::bridges::{BridgeError, BridgeRegistry, default_registry};
use super::fragments::FragmentRegistry;
use super::header::{LeanMdHeader, parse_header};
use super::macros::MacroRegistry;
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
    /// Authored macro registry — populated by `macros::extract_definitions`
    /// during each `render_body` pre-pass (spec §2.2.1).
    pub macros: RefCell<MacroRegistry>,
    /// Stack of bound `@call` param scopes (top = current macro expansion).
    /// `@if` conditions read the top scope as evalexpr variables (spec §4).
    pub param_scope: RefCell<Vec<HashMap<String, String>>>,
    /// `@import` dedupe: a library file is loaded at most once per render.
    imported: RefCell<HashSet<PathBuf>>,
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
            macros: RefCell::new(MacroRegistry::new()),
            param_scope: RefCell::new(Vec::new()),
            imported: RefCell::new(HashSet::new()),
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

    /// Push a `@call` param scope before re-entering `render_body`.
    pub fn push_params(&self, map: HashMap<String, String>) {
        self.param_scope.borrow_mut().push(map);
    }
    /// Pop the param scope after a `@call` expansion returns.
    pub fn pop_params(&self) {
        self.param_scope.borrow_mut().pop();
    }
    /// Look up a bound param in the current (top) scope.
    pub fn param(&self, name: &str) -> Option<String> {
        self.param_scope
            .borrow()
            .last()
            .and_then(|m| m.get(name).cloned())
    }
    /// Record an `@import` target; returns false if it was already imported
    /// this render (dedupe — re-entrant `render_body` must not re-load libs).
    pub fn mark_imported(&self, path: &std::path::Path) -> bool {
        self.imported.borrow_mut().insert(path.to_path_buf())
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
        let inputs = CallGraphInputs::from_project_index(&index);
        let built = Rc::new(CallGraph::load_or_build(root, &inputs));
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
    // Pass 1 (4A): strip the definition space (@define/@import) → registry.
    let body = super::macros::extract_definitions(ctx, body);
    // Pass 3 (spec §2.3): prune @if/@consumer containers → winning branch (raw).
    let body = super::macros::prune_containers(ctx, &body);
    let render = new_markdown_to_html(
        rushdown::parser::Options::default(),
        rushdown::renderer::html::Options::default(),
        lmd_parser_extension(),
        lmd_renderer_extension(ctx.clone()),
    );
    let mut out = String::new();
    let _ = render(&mut out, &body);
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
    fn define_is_invisible_in_render() {
        let out = render("@define m()\nMACRO_BODY_SENTINEL\n@define-end\n\nvisible prose\n");
        assert!(
            !out.contains("MACRO_BODY_SENTINEL") && !out.contains("@define"),
            "definition space must not appear in render: {out}"
        );
        assert!(out.contains("visible prose"), "got: {out}");
    }
    #[test]
    fn unknown_directive_renders_comment() {
        // Unregistered names fall through to the value/expr tier (Phase 4B).
        // `@frobnicate x` → resolve_value("frobnicate","x") → eval_string("frobnicate x")
        // → evalexpr error → inline error comment of the form:
        //   <!-- lmd:{{ }} eval err: Variable identifier is not bound … -->
        let out = render("@frobnicate x\n");
        assert!(
            out.contains("<!-- lmd:") && out.contains("eval err:"),
            "unknown directive must produce an eval-err comment; got: {out}"
        );
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
        crate::test_env::set_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE", "git");
        let out = render("@lean-md\nshell: allow\n\n@query git --version\n");
        crate::test_env::remove_var("LEAN_CTX_SHELL_ALLOWLIST_OVERRIDE");
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

    // ── Phase 3.4 gate: @reformat + @inspect e2e through the render pipeline ──

    #[test]
    fn reformat_renders_backend_required_e2e() {
        // @reformat must dispatch through the full render pipeline and degrade
        // to the BACKEND_REQUIRED envelope headless — never the unknown-directive
        // fallback, never a panic.
        let dir = std::env::temp_dir().join("lmd_gate_reformat");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "fn   spaced( ) {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, &format!("@reformat path={p}\n"));

        assert!(
            !out.contains("unknown directive"),
            "@reformat must dispatch (not unknown-directive fallback): {out}"
        );
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "headless reformat must degrade to BACKEND_REQUIRED envelope, got: {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
    }

    #[test]
    fn inspect_run_renders_backend_required_e2e() {
        // @inspect run <path> must dispatch and degrade cleanly headless.
        let dir = std::env::temp_dir().join("lmd_gate_inspect_run");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, &format!("@inspect run {p}\n"));

        assert!(
            !out.contains("unknown directive"),
            "@inspect must dispatch (not unknown-directive fallback): {out}"
        );
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "headless inspect run must degrade to BACKEND_REQUIRED, got: {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
    }

    #[test]
    fn inspect_list_renders_e2e() {
        // @inspect list (project-wide, no path) must dispatch — degradation or
        // a profile listing, never the unknown-directive fallback.
        let dir = std::env::temp_dir().join("lmd_gate_inspect_list");
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@inspect list\n");

        assert!(
            !out.contains("unknown directive"),
            "@inspect list must dispatch (not unknown-directive fallback): {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
    }

    // ── Phase 3.5 gate: @find + code-intel directives e2e (headless) ──

    #[test]
    fn find_renders_results_e2e() {
        let dir = std::env::temp_dir().join("lmd_gate_find");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hit.rs"), "fn gate_find_marker() {}\n").unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@find query=gate_find_marker mode=bm25\n");
        assert!(
            !out.contains("unknown directive"),
            "@find must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty @find render");
    }

    #[test]
    fn repomap_renders_e2e() {
        let dir = std::env::temp_dir().join("lmd_gate_repomap");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn gate_repo_anchor() {}\n").unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@repomap\n");
        assert!(
            !out.contains("unknown directive"),
            "@repomap must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty @repomap render");
    }

    #[test]
    fn impact_renders_e2e() {
        let dir = std::env::temp_dir().join("lmd_gate_impact");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("m.rs"),
            "fn gate_impacted() {}\nfn c() { gate_impacted(); }\n",
        )
        .unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@impact analyze path=m.rs\n");
        assert!(
            !out.contains("unknown directive"),
            "@impact must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty @impact render");
    }

    #[test]
    fn architecture_renders_e2e() {
        let dir = std::env::temp_dir().join("lmd_gate_arch");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "pub fn gate_arch_anchor() {}\n").unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@architecture overview\n");
        assert!(
            !out.contains("unknown directive"),
            "@architecture must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty @architecture render");
    }

    #[test]
    fn outline_renders_e2e() {
        let dir = std::env::temp_dir().join("lmd_gate_outline");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn gate_outline_fn() {}\n").unwrap();
        let p = f.to_str().unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, &format!("@outline path={p}\n"));
        assert!(
            !out.contains("unknown directive"),
            "@outline must dispatch: {out}"
        );
        assert!(
            out.contains("gate_outline_fn"),
            "@outline must list the symbol, got: {out}"
        );
    }

    // ── Phase 3.6 gate: @smells + @review + @routes directives e2e (headless) ──

    #[test]
    fn smells_renders_e2e() {
        let dir = std::env::temp_dir().join("lmd_gate_smells");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("s.rs"), "pub fn gate_smell_anchor() {}\n").unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@smells scan\n");
        assert!(
            !out.contains("unknown directive"),
            "@smells must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty @smells render");
    }

    #[test]
    fn review_renders_e2e() {
        // checklist is project-wide (single-line directive, no diff) — the
        // render-path-safe @review action. (diff-review needs a multi-line diff
        // → Phase-4 pipe; not render-gated here.)
        let dir = std::env::temp_dir().join("lmd_gate_review");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("r.rs"), "pub fn gate_review_anchor() {}\n").unwrap();
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@review checklist\n");
        assert!(
            !out.contains("unknown directive"),
            "@review must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty @review render");
    }

    #[test]
    fn routes_renders_e2e() {
        // ctx_routes needs an indexed file list; the self-repo crate root is not
        // indexed under nextest, so build a temp fixture with a hand-rolled
        // match-router and index it (same approach as the @routes unit test).
        let dir = std::env::temp_dir().join("lmd_gate_routes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("routes.rs"),
            "pub fn router(path: &str) {\n    match path {\n        \"/api/health\" => health(),\n        _ => {}\n    }\n}\nfn health() {}\n",
        )
            .unwrap();
        let root = dir.to_str().unwrap();
        let _ = crate::tools::ctx_impact::handle("build", None, root, None, None);
        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, "@routes path=/api/health\n");
        assert!(
            !out.contains("unknown directive"),
            "@routes must dispatch: {out}"
        );
        assert!(
            out.contains("/api/health"),
            "@routes must surface the route, got: {out}"
        );
        assert!(
            !out.contains("No routes matching"),
            "@routes render must be a real hit, not the filtered-out message: {out}"
        );
    }

    #[test]
    fn if_consumer_gates_render() {
        let out = render("@if consumer == \"human\"\nHUMAN_ONLY\n@else\nAI_PROSE\n@if-end\n");
        assert!(out.contains("AI_PROSE") && !out.contains("HUMAN_ONLY"), "got: {out}");
    }

    #[test]
    fn consumer_sugar_gates_render() {
        let out = render(
            "@lean-md\nconsumer: human\n\n@consumer human\nHUMAN_BLOCK\n@consumer-end\n",
        );
        assert!(out.contains("HUMAN_BLOCK"), "got: {out}");
    }

    #[test]
    fn if_branch_can_contain_a_directive() {
        // The surviving branch still dispatches its inner directive (pass 3 → 4).
        let out = render("@if consumer == \"ai\"\n@include hard-rules\n@if-end\n");
        assert!(out.contains("lean-ctx"), "gated @include must fire: {out}");
    }

    #[test]
    fn gated_call_macro_renders_only_in_matching_branch() {
        // Combines 4A (@call) with 4B (@if): the macro expands only for ai.
        let out = render(
            "@define note()\nGATED_NOTE\n@define-end\n\n@if consumer == \"ai\"\n@call note() /\n@if-end\n",
        );
        assert!(out.contains("GATED_NOTE"), "ai branch must expand @call: {out}");
    }

    #[test]
    fn inline_var_resolves_header_value() {
        let out = render("@lean-md 0.4\nconsumer: ai\n\nver: {{ version }}\n");
        assert!(out.contains("ver: 0.4"), "got: {out}");
    }

    #[test]
    fn inline_expr_resolves_bool() {
        crate::test_env::set_var("LMD_INLINE_FLAG", "yes");
        let out = render("flag: {{ env.LMD_INLINE_FLAG == \"yes\" }}\n");
        crate::test_env::remove_var("LMD_INLINE_FLAG");
        assert!(out.contains("flag: true"), "got: {out}");
    }

    #[test]
    fn inline_known_directive_still_dispatches() {
        // The value tier must NOT shadow a registered inline directive.
        let out = render("rules: {{ include hard-rules }}\n");
        assert!(out.contains("lean-ctx"), "got: {out}");
    }
}
