//! `@graph` Router bridge → outbound code-intelligence via ctx_graph / ctx_callgraph
//! backend calls (spec §4.5). 7 ops, no local index:
//! dependents/dependencies/related/context/recent-neighbors → ctx_graph,
//! callers/callees → ctx_callgraph.
use std::rc::Rc;

use serde_json::json;

use super::{BridgeError, DirectiveBridge};
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
        let depth = depth_arg(args);
        match op {
            // ctx_graph action=impact  — reverse-dependency tree (blast radius)
            "dependents" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                Ok(ctx
                    .backend
                    .call("ctx_graph", json!({"action":"impact","path":target,"depth":depth,"project_root":root}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
            }
            // ctx_graph action=related — forward-dependency graph
            "dependencies" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                Ok(ctx
                    .backend
                    .call("ctx_graph", json!({"action":"related","path":target,"depth":depth,"project_root":root}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
            }
            // ctx_graph action=related — bidirectional file relationships
            "related" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                Ok(ctx
                    .backend
                    .call("ctx_graph", json!({"action":"related","path":target,"depth":depth,"project_root":root}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
            }
            // ctx_callgraph action=callers — all call sites of a symbol
            "callers" => {
                let sym = args
                    .positional(1)
                    .ok_or(BridgeError::MissingArg("symbol"))?;
                Ok(ctx
                    .backend
                    .call("ctx_callgraph", json!({"action":"callers","symbol":sym,"depth":depth}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
            }
            // ctx_callgraph action=callees — all symbols called by a symbol
            "callees" => {
                let sym = args
                    .positional(1)
                    .ok_or(BridgeError::MissingArg("symbol"))?;
                Ok(ctx
                    .backend
                    .call("ctx_callgraph", json!({"action":"callees","symbol":sym,"depth":depth}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
            }
            // ctx_graph action=context — PageRank / property-graph context for a file
            "context" => {
                let target = args.positional(1).ok_or(BridgeError::MissingArg("path"))?;
                let jail_root = std::path::Path::new(root);
                // §7 PathJail: resolve the target inside the jail; an absolute
                // arg makes `join` ignore `root`, so `jail_path` is what actually
                // refuses out-of-jail and `..`-traversal paths before any read.
                let Ok(_abs) = crate::pathx::jail_path(&jail_root.join(target), jail_root) else {
                    return Ok(format!("Path '{target}' is outside the jail root"));
                };
                Ok(ctx
                    .backend
                    .call("ctx_graph", json!({"action":"context","path":target,"project_root":root}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
            }
            // ctx_graph action=neighbors — graph neighbors of one or more seed files
            "recent-neighbors" => {
                let first_seed = args.positional(1).ok_or(BridgeError::MissingArg("seed-path"))?;
                Ok(ctx
                    .backend
                    .call("ctx_graph", json!({"action":"neighbors","path":first_seed,"project_root":root}))
                    .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}")))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_is_registered() {
        assert!(super::super::default_registry().get("graph").is_some());
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
            out.contains("outside"),
            "out-of-jail path must be refused gracefully, got: {out}"
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
}
