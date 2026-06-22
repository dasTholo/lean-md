//! `@outline` bridge → symbols + signatures of one file via `ctx_outline` (v1 §4.3).
//! Headless. Read-only. `path=<P>` required (positional-0 or path=); optional
//! `kind=<filter>` narrows the symbol kinds. The backend returns (text, count) —
//! the bridge renders the text; the count is informational only.

use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

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
        let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, path)
            .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

        // Off (default): delegate to the core handler → byte-identical (E-3).
        if ctx.header.crp == crate::core::protocol::CrpMode::Off {
            let (out, _count) = crate::tools::ctx_outline::handle(&abs, kind);
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
            crate::lmd::crp::render_file_signatures(&content, ext, ctx.header.crp, kind);
        ctx.crp_sigs.borrow_mut().extend(sigs);
        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    fn ctx_with_crp(root: PathBuf, crp: crate::core::protocol::CrpMode) -> Rc<EngineContext> {
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
    fn outline_off_matches_handler_byte_identical() {
        use crate::core::protocol::CrpMode;
        let dir = std::env::temp_dir().join("lmd_outline_off");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha(x: u32) -> u32 { x }\n").unwrap();
        let abs = crate::core::path_resolve::resolve_tool_path(
            Some(dir.to_str().unwrap()),
            None,
            f.to_str().unwrap(),
        )
        .unwrap();
        let expected = crate::tools::ctx_outline::handle(&abs, None).0;

        let ctx = ctx_with_crp(dir.clone(), CrpMode::Off);
        let out = OutlineBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("path={}", f.to_str().unwrap())),
            )
            .unwrap();
        assert_eq!(out, expected, "Off must be byte-identical to the handler");
    }

    #[test]
    fn outline_tdd_emits_symbols_and_collects_sigs() {
        use crate::core::protocol::CrpMode;
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
        use crate::core::protocol::CrpMode;
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
