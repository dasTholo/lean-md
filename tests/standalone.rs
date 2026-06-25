//! #2: a code-intel-free `.lmd.md` renders fully in-process — no backend call,
//! no running lean-ctx. Proves the render core is self-contained (spec §2.2):
//! the `CodeIntelBackend` is only ever invoked by `@read`/`@refactor`/`@symbol`/…
//! directives, so a plain/markdown render never shells out.
use lean_md::render;

#[test]
fn plain_render_needs_no_backend() {
    // Pure markdown passthrough — no code-intel directive, so the backend is
    // never touched. Renders identically whether or not lean-ctx is installed.
    let out = render("# Title\n\nhello world\n");
    assert!(
        out.contains("hello world"),
        "plain render must work offline: {out}"
    );
}

#[test]
fn render_is_deterministic_offline() {
    // #498 + #2: byte-identical across calls, no backend, no timestamp leak.
    let src = "alpha beta gamma\n";
    assert_eq!(render(src), render(src));
}
