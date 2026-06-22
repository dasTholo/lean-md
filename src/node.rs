//! lmd AST nodes. `LmdDirective` is a leaf BLOCK node (`@name args`);
//! `LmdInline` is an INLINE node (`{{ name args }}`). Both store the parsed
//! name + raw args as owned strings; the renderer dispatches them into the
//! bridge registry at render time. Mirrors the Phase-0 spike's UpperBlock/ShoutInline.

use core::fmt::{self, Write};

use rushdown::ast::{KindData, NodeKind, NodeType, PrettyPrint, pp_indent};

/// Leaf-block node for an `@name args` directive line.
#[derive(Debug)]
pub struct LmdDirective {
    pub name: String,
    pub args: String,
    /// `(start, end)` byte offset of the directive text in the parsed segment
    /// source — inclusive start, exclusive end (spec D-4). Block: line span
    /// without the trailing newline. Set by the parser; used by the splice walker.
    pub span: (usize, usize),
}

impl LmdDirective {
    pub fn new(name: String, args: String, span: (usize, usize)) -> Self {
        Self { name, args, span }
    }
}

impl NodeKind for LmdDirective {
    fn typ(&self) -> NodeType {
        NodeType::LeafBlock
    }
    fn kind_name(&self) -> &'static str {
        "LmdDirective"
    }
}

impl PrettyPrint for LmdDirective {
    fn pretty_print(&self, w: &mut dyn Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(
            w,
            "{}LmdDirective: @{} {}",
            pp_indent(level),
            self.name,
            self.args
        )
    }
}

impl From<LmdDirective> for KindData {
    fn from(e: LmdDirective) -> Self {
        KindData::Extension(Box::new(e))
    }
}

/// Inline node for a `{{ name args }}` directive.
#[derive(Debug)]
pub struct LmdInline {
    pub name: String,
    pub args: String,
    /// `(start, end)` byte offset of the `{{ … }}` span in the parsed segment
    /// source — inclusive start, exclusive end (spec D-4). Set by the parser.
    pub span: (usize, usize),
}

impl LmdInline {
    pub fn new(name: String, args: String, span: (usize, usize)) -> Self {
        Self { name, args, span }
    }
}

impl NodeKind for LmdInline {
    fn typ(&self) -> NodeType {
        NodeType::Inline
    }
    fn kind_name(&self) -> &'static str {
        "LmdInline"
    }
}

impl PrettyPrint for LmdInline {
    fn pretty_print(&self, w: &mut dyn Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(
            w,
            "{}LmdInline: {{{{ {} {} }}}}",
            pp_indent(level),
            self.name,
            self.args
        )
    }
}

impl From<LmdInline> for KindData {
    fn from(e: LmdInline) -> Self {
        KindData::Extension(Box::new(e))
    }
}

/// Leaf-block node for a single pipe `@A args | @B args` (spec §5, single pipe).
#[derive(Debug)]
pub struct LmdPipe {
    pub left_name: String,
    pub left_args: String,
    pub right_name: String,
    pub right_args: String,
    /// `(start, end)` byte offset of the full pipe line in the parsed segment
    /// source — inclusive start, exclusive end (spec D-4). Line span without
    /// the trailing newline. Set by the parser; used by the splice walker.
    pub span: (usize, usize),
}

impl LmdPipe {
    pub fn new(
        left_name: String,
        left_args: String,
        right_name: String,
        right_args: String,
        span: (usize, usize),
    ) -> Self {
        Self {
            left_name,
            left_args,
            right_name,
            right_args,
            span,
        }
    }
}

impl NodeKind for LmdPipe {
    fn typ(&self) -> NodeType {
        NodeType::LeafBlock
    }
    fn kind_name(&self) -> &'static str {
        "LmdPipe"
    }
}

impl PrettyPrint for LmdPipe {
    fn pretty_print(&self, w: &mut dyn Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(
            w,
            "{}LmdPipe: @{} {} | @{} {}",
            pp_indent(level),
            self.left_name,
            self.left_args,
            self.right_name,
            self.right_args
        )
    }
}

impl From<LmdPipe> for KindData {
    fn from(e: LmdPipe) -> Self {
        KindData::Extension(Box::new(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rushdown::ast::{NodeKind, NodeType};

    #[test]
    fn directive_node_reports_leaf_block() {
        let n = LmdDirective::new("read".to_string(), "x.rs mode=full".to_string(), (0, 0));
        assert_eq!(n.typ(), NodeType::LeafBlock);
        assert_eq!(n.kind_name(), "LmdDirective");
        assert_eq!(n.name, "read");
    }

    #[test]
    fn directive_node_carries_span() {
        let n = LmdDirective::new("read".to_string(), "x.rs".to_string(), (3, 14));
        assert_eq!(n.span, (3, 14));
        assert_eq!(n.name, "read");
    }

    #[test]
    fn inline_node_reports_inline() {
        let n = LmdInline::new("include".to_string(), "hard-rules".to_string(), (0, 0));
        assert_eq!(n.typ(), NodeType::Inline);
        assert_eq!(n.kind_name(), "LmdInline");
    }

    #[test]
    fn inline_node_carries_span() {
        let n = LmdInline::new("env.CI".to_string(), String::new(), (5, 18));
        assert_eq!(n.span, (5, 18));
        assert_eq!(n.name, "env.CI");
    }

    #[test]
    fn pipe_node_reports_leaf_block() {
        let n = super::LmdPipe::new(
            "query".into(),
            "git diff".into(),
            "review".into(),
            "diff-review".into(),
            (0, 0),
        );
        assert_eq!(n.typ(), NodeType::LeafBlock);
        assert_eq!(n.kind_name(), "LmdPipe");
    }

    #[test]
    fn pipe_node_carries_span() {
        let n = super::LmdPipe::new(
            "query".into(),
            "git diff".into(),
            "review".into(),
            "diff-review".into(),
            (7, 42),
        );
        assert_eq!(n.span, (7, 42));
    }
}
