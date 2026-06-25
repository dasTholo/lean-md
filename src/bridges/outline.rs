//! `@outline` bridge → symbols + signatures of one file via `ctx_outline` (v1 §4.3).
//! Headless. Read-only. `path=<P>` required (positional-0 or path=); optional
//! `kind=<filter>` narrows the symbol kinds. The backend returns (text, count) —
//! the bridge renders the text; the count is informational only.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

pub struct OutlineBridge;

impl DirectiveBridge for OutlineBridge {
    fn name(&self) -> &'static str {
        "outline"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let path = args
            .get("path")
            .or_else(|| args.positional(0))
            .ok_or(BridgeError::MissingArg("path"))?;
        let kind = args.get("kind"); // optional kind filter

        let root = ctx.jail_root.to_str().unwrap_or(".");
        let abs = crate::pathx::resolve_tool_path(Some(root), None, path)
            .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

        // Off (default): delegate to the backend (E-3).
        if ctx.header.crp == crate::crp_proto::CrpMode::Off {
            let mut payload = serde_json::Map::new();
            payload.insert("path".into(), abs.clone().into());
            if let Some(k) = kind {
                payload.insert("kind".into(), k.into());
            }
            let out = ctx
                .backend
                .call("ctx_outline", serde_json::Value::Object(payload))
                .map_err(BridgeError::Backend)?;
            return Ok(out);
        }

        // Compact/Tdd: render notation LOCALLY (lmd edit-jail) and collect the
        // emitted signatures for the End-Hook's aggregated legend (E-4b).
        let content = std::fs::read_to_string(&abs)
            .map_err(|e| BridgeError::Resolve(format!("read {abs}: {e}")))?;
        let ext = std::path::Path::new(&abs)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let (rendered, sigs) =
            crate::crp::render_file_signatures(&content, ext, ctx.header.crp, kind);
        ctx.crp_sigs.borrow_mut().extend(sigs);
        Ok(rendered)
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

    fn ctx_with_crp(root: PathBuf, crp: crate::crp_proto::CrpMode) -> Rc<EngineContext> {
        let h = LeanMdHeader {
            crp,
            ..Default::default()
        };
        Rc::new(EngineContext::new(h, root))
    }

    #[test]
    fn outline_is_registered() {
        assert!(super::super::default_registry().get("outline").is_some());
    }

    #[test]
    fn missing_path_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = OutlineBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("path")),
            "got: {err:?}"
        );
    }

    #[test]
    fn outlines_symbols_headless() {
        let dir = std::env::temp_dir().join("lmd_outline_syms");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(
            &f,
            "pub struct OutlineAnchor;\npub fn outline_fn(x: u32) -> u32 { x }\n",
        )
        .unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("path={}", f.to_str().unwrap()));
        let out = OutlineBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "empty @outline output");
        assert!(
            out.contains("outline_fn") || out.contains("OutlineAnchor"),
            "outline must list the file's symbols, got: {out}"
        );
    }

    #[test]
    fn outline_off_dispatches_via_backend() {
        use crate::crp_proto::CrpMode;
        let dir = std::env::temp_dir().join("lmd_outline_off");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha(x: u32) -> u32 { x }\n").unwrap();

        let ctx = ctx_with_crp(dir.clone(), CrpMode::Off);
        let out = OutlineBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("path={}", f.to_str().unwrap())),
            )
            .unwrap();
        // Off mode must produce output (symbols or BACKEND_REQUIRED envelope).
        assert!(!out.trim().is_empty(), "Off must produce non-empty output");
    }

    #[test]
    fn outline_tdd_emits_symbols_and_collects_sigs() {
        use crate::crp_proto::CrpMode;
        let dir = std::env::temp_dir().join("lmd_outline_tdd");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha(x: u32) -> u32 { x }\n").unwrap();
        let ctx = ctx_with_crp(dir.clone(), CrpMode::Tdd);
        let out = OutlineBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("path={}", f.to_str().unwrap())),
            )
            .unwrap();
        assert!(out.contains("λ+alpha"), "tdd symbol form: {out}");
        assert!(
            !out.contains("λ=fn"),
            "bridge must NOT emit a legend (hook owns it): {out}"
        );
        assert!(
            !ctx.crp_sigs.borrow().is_empty(),
            "sigs collected for legend aggregation"
        );
    }

    #[test]
    fn outline_compact_emits_keyword_form() {
        use crate::crp_proto::CrpMode;
        let dir = std::env::temp_dir().join("lmd_outline_compact");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha(x: u32) -> u32 { x }\n").unwrap();
        let ctx = ctx_with_crp(dir.clone(), CrpMode::Compact);
        let out = OutlineBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("path={}", f.to_str().unwrap())),
            )
            .unwrap();
        assert!(out.contains("fn pub alpha"), "compact keyword form: {out}");
    }
}
