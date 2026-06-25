//! `@graph` Router bridge → static code-intelligence over the lean-ctx graph
//! APIs (spec §4.5). 7 ops, no LSP: dependents/dependencies/related (file deps),
//! callers/callees (call graph), context (PageRank), recent-neighbors.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::core::call_graph::CallGraph;
use crate::core::graph_context;
use crate::core::graph_index::{self, ProjectIndex};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

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
                let sym = args
                    .positional(1)
                    .ok_or(BridgeError::MissingArg("symbol"))?;
                Ok(fmt_callers(&ctx.call_graph(), sym))
            }
            "callees" => {
                let sym = args
                    .positional(1)
                    .ok_or(BridgeError::MissingArg("symbol"))?;
                Ok(fmt_callees(&ctx.call_graph(), sym))
            }
            "context" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                let jail_root = std::path::Path::new(root);
                // §7 PathJail: resolve the target inside the jail; an absolute
                // arg makes `join` ignore `root`, so `jail_path` is what actually
                // refuses out-of-jail and `..`-traversal paths before any read.
                let Ok(abs) = crate::pathx::jail_path(&jail_root.join(target), jail_root)
                else {
                    return Ok(format!("Path '{target}' is outside the jail root"));
                };
                let abs = abs.to_str().unwrap_or(target);
                match graph_context::build_graph_context(abs, root, None) {
                    Some(gc) => Ok(graph_context::format_graph_context(&gc)),
                    None => Ok(format!("No graph context available for '{target}'")),
                }
            }
            "recent-neighbors" => {
                let seeds: Vec<String> = (1..)
                    .map_while(|i| args.positional(i))
                    .map(|p| graph_index::graph_relative_key(p, root))
                    .collect();
                if seeds.is_empty() {
                    return Err(BridgeError::MissingArg("seed-path"));
                }
                Ok(fmt_recent_neighbors(root, &seeds))
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

/// Render the rank map (lower rank = closer neighbor) as a sorted list.
fn fmt_recent_neighbors(root: &str, seeds: &[String]) -> String {
    match graph_context::graph_neighbor_ranks_for_recent_files(root, seeds, 10, 20) {
        Some(ranks) if !ranks.is_empty() => {
            let mut entries: Vec<(&String, &usize)> = ranks.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)));
            let mut out = format!(
                "{} graph neighbor(s) of {} recent seed(s):\n",
                entries.len(),
                seeds.len()
            );
            for (path, rank) in entries {
                out.push_str(&format!("  [{rank}] {path}\n"));
            }
            out
        }
        _ => format!("No graph neighbors for {} seed(s)", seeds.len()),
    }
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
    fn context_op_renders_for_a_real_file() {
        use crate::header::LeanMdHeader;
        use std::path::PathBuf;
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let args = DirectiveArgs::parse("context rust/src/lmd/engine.rs");
        let out = GraphBridge
            .execute(&ctx, &args)
            .expect("context op must not error");
        // Either a rendered context or a graceful "no context" line — never empty.
        assert!(!out.trim().is_empty(), "got empty output");
    }

    #[test]
    fn context_op_rejects_out_of_jail_path() {
        use crate::header::LeanMdHeader;
        use std::path::PathBuf;
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        // Absolute path outside the jail root must be refused, not read.
        let args = DirectiveArgs::parse("context /etc/passwd");
        let out = GraphBridge.execute(&ctx, &args).expect("must not error");
        assert!(
            out.contains("outside") || out.contains("No graph context"),
            "out-of-jail path must be refused gracefully, got: {out}"
        );
        assert!(
            !out.contains("root:"),
            "must not leak /etc/passwd content, got: {out}"
        );
    }

    #[test]
    fn recent_neighbors_requires_at_least_one_seed() {
        use crate::header::LeanMdHeader;
        use std::path::PathBuf;
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let err = GraphBridge
            .execute(&ctx, &DirectiveArgs::parse("recent-neighbors"))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)), "got: {err:?}");
    }

    #[test]
    fn recent_neighbors_renders_for_real_seed() {
        use crate::header::LeanMdHeader;
        use std::path::PathBuf;
        let ctx = Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ));
        let args = DirectiveArgs::parse("recent-neighbors rust/src/lmd/engine.rs");
        let out = GraphBridge.execute(&ctx, &args).expect("must not error");
        assert!(!out.trim().is_empty(), "got empty output");
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
