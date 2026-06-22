//! Render-time bridge dispatch: `splice_directives` walks the parsed arena and
//! replaces each `Lmd*` directive span with its bridge output, leaving all
//! other source bytes byte-identical. `dispatch*`/`resolve_value`/
//! `sanitize_comment` produce the per-directive output strings.

use std::rc::Rc;

use super::args::DirectiveArgs;
use super::engine::EngineContext;
use super::node::{LmdDirective, LmdInline, LmdPipe};
use super::parser::lmd_parser_extension;

/// Neutralize HTML-comment delimiters so an untrusted directive name or a
/// bridge err str cannot break out of the fallback `<!-- … -->` wrapper
/// (spec §9 F-2). Phase-1 target is the AI ctx, not a browser DOM, so a
/// minimal delimiter-escape is sufficient.
fn sanitize_comment(s: &str) -> String {
    s.replace("-->", "--&gt;").replace("<!--", "&lt;!--")
}

/// Result-returning dispatch sibling: the phase executor needs the `Err` to
/// drive abort (spec §3.2). A non-directive name falls back to the value tier
/// (always `Ok`). Bridges that error surface the `BridgeError` verbatim.
pub(crate) fn dispatch_result(
    ctx: &Rc<EngineContext>,
    name: &str,
    raw_args: &str,
) -> std::result::Result<String, super::bridges::BridgeError> {
    let args = DirectiveArgs::parse(raw_args);
    match ctx.registry.get(name) {
        Some(bridge) => bridge.execute(ctx, &args),
        None => Ok(resolve_value(ctx, name, raw_args)),
    }
}

/// Look up `name` in the registry and run the bridge; on miss/error emit a
/// visible comment instead of failing the whole render.
fn dispatch(ctx: &Rc<EngineContext>, name: &str, raw_args: &str) -> String {
    match dispatch_result(ctx, name, raw_args) {
        Ok(out) => out,
        Err(e) => format!(
            "<!-- lmd:@{} err: {} -->",
            sanitize_comment(name),
            sanitize_comment(&format!("{e:?}"))
        ),
    }
}

/// Inline `{{ … }}` value tier (spec §3.1): a non-directive name resolves as a
/// bound macro-param, then a header var / evalexpr expression. The full
/// `{{ name args }}` text is reconstructed so multi-token exprs
/// (`{{ env.CI == "true" }}`) evaluate as one expression.
fn resolve_value(ctx: &Rc<EngineContext>, name: &str, raw_args: &str) -> String {
    if raw_args.is_empty()
        && let Some(v) = ctx.param(name)
    {
        return v;
    }
    let expr = if raw_args.is_empty() {
        name.to_string()
    } else {
        format!("{name} {raw_args}")
    };
    crate::lmd::macros::eval_string(ctx, &expr)
}

/// Dispatch a single pipe: run left (no piped input), inject its output as the
/// right side's `piped_input`, then run right. Only the right output is
/// returned (spec §5: the raw left intermediate is consumed, not rendered).
/// Piping into a bridge that does not `accepts_pipe()` is a visible error.
fn dispatch_pipe(
    ctx: &Rc<EngineContext>,
    left_name: &str,
    left_args: &str,
    right_name: &str,
    right_args: &str,
) -> String {
    let left_out = dispatch(ctx, left_name, left_args);
    match ctx.registry.get(right_name) {
        Some(bridge) if bridge.accepts_pipe() => {
            let args = DirectiveArgs::parse(right_args).with_piped_input(left_out);
            match bridge.execute(ctx, &args) {
                Ok(out) => out,
                Err(e) => format!(
                    "<!-- lmd:@{} err: {} -->",
                    sanitize_comment(right_name),
                    sanitize_comment(&format!("{e:?}"))
                ),
            }
        }
        Some(_) => format!(
            "<!-- lmd: @{} does not accept piped input -->",
            sanitize_comment(right_name)
        ),
        None => format!(
            "<!-- lmd: unknown directive @{} -->",
            sanitize_comment(right_name)
        ),
    }
}

/// Source-preserving directive expander (spec K2). DFS-collects every `Lmd*`
/// node as `(span, bridge_output)`, then splices the outputs into `source`,
/// leaving all non-directive bytes byte-identical. Overlapping spans (defensive;
/// directives don't nest) are skipped deterministically. Pure string op over
/// already-gated bridge outputs — no new attack surface (spec §6).
pub(crate) fn splice_directives(
    ctx: &Rc<EngineContext>,
    source: &str,
    arena: &rushdown::ast::Arena,
    root: rushdown::ast::NodeRef,
) -> String {
    use core::fmt;
    use rushdown::ast::{self, WalkStatus};
    use rushdown::{as_extension_data, matches_extension_kind};

    #[derive(Debug)]
    struct WalkError(String);
    impl fmt::Display for WalkError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl std::error::Error for WalkError {}

    let mut edits: Vec<((usize, usize), String)> = Vec::new();
    let _ = ast::walk::<WalkError>(arena, root, &mut |a: &ast::Arena,
                                                      nref: ast::NodeRef,
                                                      entering: bool|
     -> std::result::Result<
        WalkStatus,
        WalkError,
    > {
        if entering {
            if matches_extension_kind!(a, nref, LmdDirective) {
                let d = as_extension_data!(a, nref, LmdDirective);
                edits.push((d.span, dispatch(ctx, &d.name, &d.args)));
            } else if matches_extension_kind!(a, nref, LmdInline) {
                let d = as_extension_data!(a, nref, LmdInline);
                edits.push((d.span, dispatch(ctx, &d.name, &d.args)));
            } else if matches_extension_kind!(a, nref, LmdPipe) {
                let p = as_extension_data!(a, nref, LmdPipe);
                edits.push((
                    p.span,
                    dispatch_pipe(
                        ctx,
                        &p.left_name,
                        &p.left_args,
                        &p.right_name,
                        &p.right_args,
                    ),
                ));
            }
        }
        Ok(WalkStatus::Continue)
    });

    edits.sort_by_key(|((s, _), _)| *s);

    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for ((start, end), replacement) in edits {
        // `start < cursor` also resolves the duplicate-start case: edits sharing
        // a start offset are kept in DFS order by the stable sort, so the first
        // one wins and any later duplicate falls through here (its start is now
        // < cursor). Directives don't nest, so this is defensive only.
        if start < cursor || end < start || end > source.len() {
            continue; // overlap / invalid span — skip deterministically
        }
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            debug_assert!(
                false,
                "lmd splice span not on char boundary: {start}..{end}"
            );
            continue;
        }
        out.push_str(&source[cursor..start]);
        out.push_str(&replacement);
        cursor = end;
    }
    out.push_str(&source[cursor..]);
    out
}

/// Work-Klasse (Spec D-3): diese Direktiven führt der SUBAGENT in seinem Kontext
/// aus → beim `@dispatch`-Render bleiben sie VERBATIM (lazy). Alles andere
/// (`@call`/`@include` + `{{ }}`-Inline) ist Template-Klasse → eager aufgelöst.
const WORK_DIRECTIVES: &[&str] = &[
    "read",
    "search",
    "edit",
    "symbol",
    "graph",
    "query",
    "find",
    "impact",
    "repomap",
    "outline",
    "architecture",
    "routes",
    "smells",
    "review",
    "inspect",
    "count",
    "refactor",
    "reformat",
    "list",
];

/// Template-eager / Work-lazy splice (Spec D-3): wie `splice_directives`, aber
/// Work-Direktiven (`WORK_DIRECTIVES`) und Pipes werden NICHT dispatcht — ihr
/// Source-Span bleibt byte-identisch. Nur Template-Direktiven (`@call`/`@include`)
/// und `{{ }}`-Inlines werden aufgelöst.
pub(crate) fn splice_template_only(ctx: &Rc<EngineContext>, segment: &str) -> String {
    use core::fmt;
    use rushdown::ast::{self, WalkStatus};
    use rushdown::parser::{Options as ParserOptions, Parser};
    use rushdown::text::BasicReader;
    use rushdown::{as_extension_data, matches_extension_kind};

    #[derive(Debug)]
    struct WalkError(String);
    impl fmt::Display for WalkError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl std::error::Error for WalkError {}

    let parser = Parser::with_extensions(ParserOptions::default(), lmd_parser_extension());
    let mut reader = BasicReader::new(segment);
    let (arena, root) = parser.parse(&mut reader);

    let mut edits: Vec<((usize, usize), String)> = Vec::new();
    let _ = ast::walk::<WalkError>(&arena, root, &mut |a: &ast::Arena,
                                                       nref: ast::NodeRef,
                                                       entering: bool|
     -> std::result::Result<
        WalkStatus,
        WalkError,
    > {
        if entering {
            if matches_extension_kind!(a, nref, LmdDirective) {
                let d = as_extension_data!(a, nref, LmdDirective);
                if !WORK_DIRECTIVES.contains(&d.name.as_str()) {
                    // Template (@call/@include/…) → eager dispatch.
                    edits.push((d.span, dispatch(ctx, &d.name, &d.args)));
                }
                // else: Work → no edit → source span stays verbatim.
            } else if matches_extension_kind!(a, nref, LmdInline) {
                // `{{ }}` is template-class → always resolve eager.
                let d = as_extension_data!(a, nref, LmdInline);
                edits.push((d.span, dispatch(ctx, &d.name, &d.args)));
            }
            // LmdPipe: conservative → verbatim (no edit).
        }
        Ok(WalkStatus::Continue)
    });

    edits.sort_by_key(|((s, _), _)| *s);
    let mut out = String::with_capacity(segment.len());
    let mut cursor = 0usize;
    for ((start, end), replacement) in edits {
        if start < cursor || end < start || end > segment.len() {
            continue;
        }
        if !segment.is_char_boundary(start) || !segment.is_char_boundary(end) {
            continue;
        }
        out.push_str(&segment[cursor..start]);
        out.push_str(&replacement);
        cursor = end;
    }
    out.push_str(&segment[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use rushdown::parser::{Options as ParserOptions, Parser};
    use rushdown::text::BasicReader;

    use super::{sanitize_comment, splice_directives};
    use crate::lmd::engine::EngineContext;
    use crate::lmd::header::LeanMdHeader;
    use crate::lmd::parser::lmd_parser_extension;

    #[test]
    fn sanitizes_comment_breakout_sequences() {
        assert_eq!(sanitize_comment("x-->y"), "x--&gt;y");
        assert_eq!(sanitize_comment("<!--z"), "&lt;!--z");
        assert_eq!(sanitize_comment("plain"), "plain");
    }

    #[test]
    fn splice_passthrough_without_directives() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::path::PathBuf::from("."),
        ));
        let source = "# Title\n\nPlain *prose* and a list:\n- a\n- b\n";
        let parser = Parser::with_extensions(ParserOptions::default(), lmd_parser_extension());
        let mut reader = BasicReader::new(source);
        let (arena, root) = parser.parse(&mut reader);
        let out = splice_directives(&ctx, source, &arena, root);
        assert_eq!(out, source, "no directives => byte-identical passthrough");
    }

    #[test]
    fn splice_replaces_directive_only() {
        let f = std::env::temp_dir().join("lmd_splice_t3.txt");
        std::fs::write(&f, "SPLICE_SENTINEL_3\n").unwrap();
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        // @read is a block directive — must appear at line-start. Prose lives in
        // a separate paragraph so the surrounding bytes are byte-preserved.
        let source = format!(
            "pre prose\n\n@read {}\n\npost unrelated\n",
            f.to_str().unwrap()
        );
        let parser = Parser::with_extensions(ParserOptions::default(), lmd_parser_extension());
        let mut reader = BasicReader::new(&source);
        let (arena, root) = parser.parse(&mut reader);
        let out = splice_directives(&ctx, &source, &arena, root);
        assert!(
            out.contains("SPLICE_SENTINEL_3"),
            "bridge output spliced in: {out}"
        );
        assert!(
            out.contains("pre prose"),
            "preceding prose byte-preserved: {out}"
        );
        assert!(
            out.contains("post unrelated"),
            "following prose byte-preserved: {out}"
        );
        assert!(!out.contains("<p>"), "no HTML leak: {out}");
    }

    #[test]
    fn splice_preserves_multibyte_prose_around_directive() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        let source = "Grüße {{ date }} — Größe äöü\n";
        let parser = Parser::with_extensions(ParserOptions::default(), lmd_parser_extension());
        let mut reader = BasicReader::new(source);
        let (arena, root) = parser.parse(&mut reader);
        let out = splice_directives(&ctx, source, &arena, root);
        assert!(
            out.starts_with("Grüße "),
            "leading multibyte prose preserved: {out}"
        );
        assert!(
            out.contains("Größe äöü"),
            "trailing multibyte prose preserved: {out}"
        );
    }

    #[test]
    fn splice_directive_on_second_line_keeps_offset() {
        let f = std::env::temp_dir().join("lmd_splice_t3_line2.txt");
        std::fs::write(&f, "SENTINEL_LINE2\n").unwrap();
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        let source = format!("# Heading\n\n@read {}\n\ntail prose\n", f.to_str().unwrap());
        let parser = Parser::with_extensions(ParserOptions::default(), lmd_parser_extension());
        let mut reader = BasicReader::new(&source);
        let (arena, root) = parser.parse(&mut reader);
        let out = splice_directives(&ctx, &source, &arena, root);
        assert!(
            out.starts_with("# Heading\n\n"),
            "prose before directive byte-preserved: {out:?}"
        );
        assert!(
            out.contains("SENTINEL_LINE2"),
            "bridge output present at correct position: {out:?}"
        );
        assert!(
            out.contains("tail prose"),
            "prose after directive byte-preserved: {out:?}"
        );
        assert!(!out.contains("<p>"), "no HTML leak: {out}");
    }

    #[test]
    fn template_only_keeps_work_bridges_verbatim() {
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::path::PathBuf::from("."),
        ));
        // @include = template (resolved); @read/@query = work (verbatim).
        let seg = "@include hard-rules\n@read src/x.rs\n@query \"cargo nextest run\"\n";
        let out = super::splice_template_only(&ctx, seg);
        // Work bridges stay verbatim (lazy, D-3):
        assert!(
            out.contains("@read src/x.rs"),
            "work @read must stay verbatim: {out}"
        );
        assert!(
            out.contains("@query \"cargo nextest run\""),
            "work @query verbatim: {out}"
        );
        // Template @include is resolved (hard-rules content present, directive gone):
        assert!(
            out.contains("lean-ctx"),
            "@include hard-rules must resolve: {out}"
        );
        assert!(
            !out.contains("@include hard-rules"),
            "template @include must be consumed: {out}"
        );
    }

    #[test]
    fn splice_inline_on_later_line_keeps_offset() {
        // Inline `{{ … }}` on line 3: lines 1-2 and the rest of line 3 plus the
        // trailing lines must stay byte-identical; only the `{{ … }}` span is
        // replaced. Closes the coverage gap for inline directives on non-first
        // segment lines (block line-2 is covered by the test above). A bound
        // param gives the inline a deterministic, distinct output so the test
        // genuinely exercises the replacement (no silent passthrough no-op).
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            std::env::temp_dir(),
        ));
        let mut params = std::collections::HashMap::new();
        params.insert("inlinevar".to_string(), "INLINE_VAL".to_string());
        ctx.push_params(params);
        let source = "# Heading\n\nValue is {{ inlinevar }} here\n\ntail prose\n";
        let parser = Parser::with_extensions(ParserOptions::default(), lmd_parser_extension());
        let mut reader = BasicReader::new(source);
        let (arena, root) = parser.parse(&mut reader);
        let out = splice_directives(&ctx, source, &arena, root);
        assert!(
            out.starts_with("# Heading\n\nValue is "),
            "leading lines + line-3 prefix byte-preserved: {out:?}"
        );
        assert!(
            out.contains("INLINE_VAL"),
            "inline resolved + spliced at the correct offset: {out:?}"
        );
        assert!(
            out.ends_with(" here\n\ntail prose\n"),
            "line-3 suffix + trailing lines byte-preserved: {out:?}"
        );
        assert!(
            !out.contains("{{ inlinevar }}"),
            "inline span was actually replaced: {out:?}"
        );
    }
}
