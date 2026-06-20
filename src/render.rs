//! Render-time bridge dispatch. The renderer hook carries `Rc<EngineContext>`
//! (incl. the bridge registry) via a custom `LmdRendererOptions` (rushdown's
//! `RendererOptions` is an empty marker trait), then dispatches each lmd node
//! into the registry. Phase 1 writes bridge output raw via write_html.
//! Mirrors the Phase-0 spike renderer structs.

use core::any::TypeId;
use std::rc::Rc;

use rushdown::{
    Result, as_extension_data,
    ast::{Arena, NodeRef, WalkStatus},
    renderer,
    renderer::{
        BoxRenderNode, NodeRenderer, NodeRendererRegistry, RenderNode, RendererOptions, TextWrite,
        html,
        html::{Options, RendererExtension, renderer_extension},
    },
};

use super::args::DirectiveArgs;
use super::engine::EngineContext;
use super::node::{LmdDirective, LmdInline};

/// Carries the engine context into the render hook.
pub struct LmdRendererOptions {
    pub ctx: Rc<EngineContext>,
}

impl RendererOptions for LmdRendererOptions {}

/// Neutralize HTML-comment delimiters so an untrusted directive name or a
/// bridge err str cannot break out of the fallback `<!-- … -->` wrapper
/// (spec §9 F-2). Phase-1 target is the AI ctx, not a browser DOM, so a
/// minimal delimiter-escape is sufficient.
fn sanitize_comment(s: &str) -> String {
    s.replace("-->", "--&gt;").replace("<!--", "&lt;!--")
}

/// Look up `name` in the registry and run the bridge; on miss/error emit a
/// visible comment instead of failing the whole render.
fn dispatch(ctx: &Rc<EngineContext>, name: &str, raw_args: &str) -> String {
    let args = DirectiveArgs::parse(raw_args);
    match ctx.registry.get(name) {
        Some(bridge) => match bridge.execute(ctx, &args) {
            Ok(out) => out,
            Err(e) => format!(
                "<!-- lmd:@{} err: {} -->",
                sanitize_comment(name),
                sanitize_comment(&format!("{e:?}"))
            ),
        },
        None => format!(
            "<!-- lmd: unknown directive @{} -->",
            sanitize_comment(name)
        ),
    }
}

pub struct LmdDirectiveRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
    ctx: Rc<EngineContext>,
}

impl<W: TextWrite> LmdDirectiveRenderer<W> {
    fn with_options(html_opts: Options, options: LmdRendererOptions) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
            ctx: options.ctx,
        }
    }
}

impl<W: TextWrite> RenderNode<W> for LmdDirectiveRenderer<W> {
    fn render_node<'a>(
        &self,
        w: &mut W,
        _source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            let d = as_extension_data!(arena, node_ref, LmdDirective);
            let out = dispatch(&self.ctx, &d.name, &d.args);
            // `write_html` passes the bytes through verbatim (only null-char
            // sanitization), i.e. RAW HTML passthrough for dynamic bridge output.
            // `write_safe_str` is unusable here: its `SafeStr` bound is sealed to
            // `&'static str`, so a transient `String` cannot be written through it.
            self.writer.write_html(w, &out)?;
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'cb, W> NodeRenderer<'cb, W> for LmdDirectiveRenderer<W>
where
    W: TextWrite + 'cb,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'cb, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<LmdDirective>(), BoxRenderNode::new(self));
    }
}

pub struct LmdInlineRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
    ctx: Rc<EngineContext>,
}

impl<W: TextWrite> LmdInlineRenderer<W> {
    fn with_options(html_opts: Options, options: LmdRendererOptions) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
            ctx: options.ctx,
        }
    }
}

impl<W: TextWrite> RenderNode<W> for LmdInlineRenderer<W> {
    fn render_node<'a>(
        &self,
        w: &mut W,
        _source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            let d = as_extension_data!(arena, node_ref, LmdInline);
            let out = dispatch(&self.ctx, &d.name, &d.args);
            // `write_html` passes the bytes through verbatim (only null-char
            // sanitization), i.e. RAW HTML passthrough for dynamic bridge output.
            // `write_safe_str` is unusable here: its `SafeStr` bound is sealed to
            // `&'static str`, so a transient `String` cannot be written through it.
            self.writer.write_html(w, &out)?;
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'cb, W> NodeRenderer<'cb, W> for LmdInlineRenderer<W>
where
    W: TextWrite + 'cb,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'cb, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<LmdInline>(), BoxRenderNode::new(self));
    }
}

/// Registers both lmd node renderers, each carrying a clone of the engine ctx.
pub fn lmd_renderer_extension<'cb, W>(ctx: Rc<EngineContext>) -> impl RendererExtension<'cb, W>
where
    W: TextWrite + 'cb,
{
    renderer_extension(move |r| {
        r.add_node_renderer(
            LmdDirectiveRenderer::with_options,
            LmdRendererOptions { ctx: ctx.clone() },
        );
        r.add_node_renderer(
            LmdInlineRenderer::with_options,
            LmdRendererOptions { ctx: ctx.clone() },
        );
    })
}

#[cfg(test)]
mod tests {
    use super::sanitize_comment;

    #[test]
    fn sanitizes_comment_breakout_sequences() {
        assert_eq!(sanitize_comment("x-->y"), "x--&gt;y");
        assert_eq!(sanitize_comment("<!--z"), "&lt;!--z");
        assert_eq!(sanitize_comment("plain"), "plain");
    }
}
