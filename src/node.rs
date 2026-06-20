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
}

impl LmdDirective {
    pub fn new(name: String, args: String) -> Self {
        Self { name, args }
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
}

impl LmdInline {
    pub fn new(name: String, args: String) -> Self {
        Self { name, args }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rushdown::ast::{NodeKind, NodeType};

    #[test]
    fn directive_node_reports_leaf_block() {
        let n = LmdDirective::new("read".to_string(), "x.rs mode=full".to_string());
        assert_eq!(n.typ(), NodeType::LeafBlock);
        assert_eq!(n.kind_name(), "LmdDirective");
        assert_eq!(n.name, "read");
    }

    #[test]
    fn inline_node_reports_inline() {
        let n = LmdInline::new("include".to_string(), "hard-rules".to_string());
        assert_eq!(n.typ(), NodeType::Inline);
        assert_eq!(n.kind_name(), "LmdInline");
    }
}
