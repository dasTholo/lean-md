//! `@graph` Router bridge → static code-intelligence over the lean-ctx graph
//! APIs (spec §4.5). 7 ops, no LSP: dependents/dependencies/related (file deps),
//! callers/callees (call graph), context (PageRank), recent-neighbors.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::core::graph_index::{self, ProjectIndex};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct GraphBridge;

impl DirectiveBridge for GraphBridge {
    fn name(&self) -> &'static str {
        "graph"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
        let root = ctx.jail_root.to_str().unwrap_or(".");
        match op {
            "dependents" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                let key = graph_index::graph_relative_key(target, root);
                Ok(fmt_dependents(&ctx.index(), &key, depth_arg(args)))
            }
            "dependencies" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                let key = graph_index::graph_relative_key(target, root);
                Ok(fmt_dependencies(&ctx.index(), &key, depth_arg(args)))
            }
            "related" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                let key = graph_index::graph_relative_key(target, root);
                Ok(fmt_related(&ctx.index(), &key, depth_arg(args)))
            }
            other => Err(BridgeError::Resolve(format!(
                "unknown @graph op '{other}'. Use: dependents|dependencies|related|callers|callees|context|recent-neighbors"
            ))),
        }
    }
}

/// `depth=N` named arg, default 2, clamped to 1..=5.
fn depth_arg(args: &DirectiveArgs) -> usize {
    args.get("depth")
        .and_then(|d| d.parse::<usize>().ok())
        .unwrap_or(2)
        .clamp(1, 5)
}

fn fmt_dependents(index: &ProjectIndex, key: &str, depth: usize) -> String {
    let deps = index.get_reverse_deps(key, depth);
    if deps.is_empty() {
        return format!(
            "No dependents of '{key}' ({} files, {} edges indexed)",
            index.file_count(),
            index.edge_count()
        );
    }
    let mut out = format!("{} dependent(s) of '{key}' (depth≤{depth}):\n", deps.len());
    for d in &deps {
        out.push_str(&format!("  {d}\n"));
    }
    out
}

fn fmt_dependencies(index: &ProjectIndex, key: &str, depth: usize) -> String {
    let deps = index.get_forward_deps(key, depth);
    if deps.is_empty() {
        return format!(
            "No dependencies of '{key}' ({} files, {} edges indexed)",
            index.file_count(),
            index.edge_count()
        );
    }
    let mut out = format!(
        "{} dependenc(ies) of '{key}' (depth≤{depth}):\n",
        deps.len()
    );
    for d in &deps {
        out.push_str(&format!("  {d}\n"));
    }
    out
}

fn fmt_related(index: &ProjectIndex, key: &str, depth: usize) -> String {
    let rel = index.get_related(key, depth);
    if rel.is_empty() {
        return format!(
            "No related files for '{key}' ({} files, {} edges indexed)",
            index.file_count(),
            index.edge_count()
        );
    }
    let mut out = format!(
        "{} related file(s) for '{key}' (depth≤{depth}):\n",
        rel.len()
    );
    for d in &rel {
        out.push_str(&format!("  {d}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::graph_index::IndexEdge;

    fn index_a_imports_b() -> ProjectIndex {
        let mut idx = ProjectIndex::new("/tmp/g");
        idx.edges.push(IndexEdge {
            from: "a.rs".into(),
            to: "b.rs".into(),
            kind: "import".into(),
            weight: 1.0,
        });
        idx
    }

    #[test]
    fn fmt_dependents_lists_importers() {
        let out = fmt_dependents(&index_a_imports_b(), "b.rs", 2);
        assert!(out.contains("a.rs"), "got: {out}");
        assert!(out.contains("dependent"), "got: {out}");
    }

    #[test]
    fn fmt_dependents_empty_is_explained() {
        let out = fmt_dependents(&index_a_imports_b(), "nope.rs", 2);
        assert!(out.contains("No dependents"), "got: {out}");
    }

    #[test]
    fn graph_is_registered() {
        assert!(super::super::default_registry().get("graph").is_some());
    }

    #[test]
    fn fmt_dependencies_lists_imported() {
        let out = fmt_dependencies(&index_a_imports_b(), "a.rs", 2);
        assert!(out.contains("b.rs"), "got: {out}");
        assert!(out.contains("dependenc"), "got: {out}");
    }

    #[test]
    fn fmt_related_lists_either_direction() {
        let out = fmt_related(&index_a_imports_b(), "a.rs", 2);
        assert!(out.contains("b.rs"), "got: {out}");
    }
}
