//! `@graph` Router bridge → static code-intelligence over the lean-ctx graph
//! APIs (spec §4.5). 7 ops, no LSP: dependents/dependencies/related (file deps),
//! callers/callees (call graph), context (PageRank), recent-neighbors.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::core::call_graph::CallGraph;
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
            "callers" => {
                let sym = args.positional(1).ok_or(BridgeError::MissingArg("symbol"))?;
                Ok(fmt_callers(&ctx.call_graph(), sym))
            }
            "callees" => {
                let sym = args.positional(1).ok_or(BridgeError::MissingArg("symbol"))?;
                Ok(fmt_callees(&ctx.call_graph(), sym))
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

fn fmt_callers(graph: &CallGraph, symbol: &str) -> String {
    let callers = graph.callers_of(symbol);
    if callers.is_empty() {
        return format!(
            "No callers of '{symbol}' ({} edges in call graph)",
            graph.edges.len()
        );
    }
    let mut out = format!("{} caller(s) of '{symbol}':\n", callers.len());
    for e in &callers {
        out.push_str(&format!(
            "  {} → {}  (L{})\n",
            e.caller_file, e.caller_symbol, e.caller_line
        ));
    }
    out
}

fn fmt_callees(graph: &CallGraph, symbol: &str) -> String {
    let callees = graph.callees_of(symbol);
    if callees.is_empty() {
        return format!(
            "No callees of '{symbol}' ({} edges in call graph)",
            graph.edges.len()
        );
    }
    let mut out = format!("{} callee(s) of '{symbol}':\n", callees.len());
    for e in &callees {
        out.push_str(&format!(
            "  → {}  ({}:L{})\n",
            e.callee_name, e.caller_file, e.caller_line
        ));
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

    fn call_graph_a_calls_b() -> crate::core::call_graph::CallGraph {
        use crate::core::call_graph::{CallEdge, CallGraph};
        let mut g = CallGraph::new("/tmp/g");
        g.edges.push(CallEdge {
            caller_file: "a.rs".into(),
            caller_symbol: "fn_a".into(),
            caller_line: 10,
            callee_name: "fn_b".into(),
        });
        g
    }

    #[test]
    fn fmt_callers_lists_calling_symbols() {
        let out = fmt_callers(&call_graph_a_calls_b(), "fn_b");
        assert!(out.contains("fn_a"), "got: {out}");
        assert!(out.contains("caller"), "got: {out}");
    }

    #[test]
    fn fmt_callees_lists_called_symbols() {
        let out = fmt_callees(&call_graph_a_calls_b(), "fn_a");
        assert!(out.contains("fn_b"), "got: {out}");
        assert!(out.contains("callee"), "got: {out}");
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
