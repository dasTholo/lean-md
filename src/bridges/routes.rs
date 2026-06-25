//! `@routes` bridge → HTTP route extraction via `ctx_routes` (v1 §4.4).
//! Headless. Read-only. No action concept: `method=` and `path=` are FILTERS.
//! `path=` is an HTTP route PREFIX (e.g. `/api`), NOT a filesystem path — it is
//! NOT jail-resolved. Without filters, all routes are listed. Backends covered:
//! Express/Flask/FastAPI/Actix/Spring/Rails/Next.js + axum + hand-rolled match routers.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct RoutesBridge;

impl DirectiveBridge for RoutesBridge {
    fn name(&self) -> &'static str {
        "routes"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let method = args.get("method"); // optional GET|POST|… filter
        let path_prefix = args.get("path"); // ROUTE prefix filter — NOT an FS path, no jail

        let mut payload = serde_json::Map::new();
        if let Some(m) = method {
            payload.insert("method".into(), m.into());
        }
        if let Some(p) = path_prefix {
            payload.insert("path".into(), p.into());
        }
        let out = ctx
            .backend
            .call("ctx_routes", serde_json::Value::Object(payload))
            .unwrap_or_else(|e| format!("ERROR: BACKEND_REQUIRED: {e}"));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    /// Write a small fixture with a hand-rolled match-router and build its graph
    /// so `ctx_routes` (via `open_or_build → open_existing`) sees a real file list.
    /// `ctx_routes` needs the project's indexed file list; the self-repo crate root
    /// is not indexed under nextest, so we index a temp fixture explicitly — the
    /// same approach the `@smells` scan test uses.
    fn fixture_with_built_graph(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("routes.rs"),
            "pub fn router(path: &str) {\n    match path {\n        \"/api/health\" => health(),\n        _ => {}\n    }\n}\nfn health() {}\n",
        )
        .unwrap();
        let root = dir.to_str().unwrap();
        // Build the graph so the route file appears in the indexed file list.
        let backend = crate::backend::default_backend(root);
        let mut build_payload = serde_json::Map::new();
        build_payload.insert("action".into(), "build".into());
        let _ = backend.call("ctx_impact", serde_json::Value::Object(build_payload));
        dir
    }

    #[test]
    fn routes_is_registered() {
        assert!(super::super::default_registry().get("routes").is_some());
    }

    #[test]
    fn routes_extracts_real_route_from_built_graph() {
        // Genuine positive: the success format is "N route(s):\n  * /api/health … (routes.rs:L…)".
        // Asserting "route(s):" rules out the vacuous "No routes matching '/api/health'" echo.
        let dir = fixture_with_built_graph("lmd_routes_fixture_pos");
        let ctx = ctx_at(dir);
        let out = RoutesBridge
            .execute(&ctx, &DirectiveArgs::parse("path=/api/health"))
            .unwrap();
        assert!(
            out.contains("route(s):") && out.contains("/api/health"),
            "@routes must surface the extracted /api/health route, got: {out}"
        );
    }

    #[test]
    fn routes_filter_excludes_nonmatching_prefix() {
        // Genuine negative: a non-matching prefix yields the "No routes matching"
        // message (the route IS extracted but filtered out) — not "No HTTP routes
        // found" (which would mean the graph was empty). Asserting "No routes
        // matching" therefore proves both extraction AND the filter ran.
        let dir = fixture_with_built_graph("lmd_routes_fixture_neg");
        let ctx = ctx_at(dir);
        let out = RoutesBridge
            .execute(&ctx, &DirectiveArgs::parse("path=/api/zzz-nonexistent"))
            .unwrap();
        assert!(
            out.contains("No routes matching"),
            "a non-matching prefix must yield the negative filter message, got: {out}"
        );
    }
}
