//! Engine wiring: header pre-scan → build the rushdown render closure with the
//! lmd parser + renderer extensions → render the body. `render_body` is the
//! re-entry point used by `@include` for recursive fragment rendering.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::core::cache::SessionCache;
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
}
