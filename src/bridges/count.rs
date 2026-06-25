//! `@count` Router bridge → glob match count (Spec §3.1 trivial-R).
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@count <glob>` — number of filesystem paths matching the glob pattern.
/// Paths that cannot be read (e.g. permission-denied during traversal) are
/// silently excluded from the count.
pub struct CountBridge;

impl DirectiveBridge for CountBridge {
    fn name(&self) -> &'static str {
        "count"
    }

    fn execute(
        &self,
        _ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let pattern = args
            .positional(0)
            .ok_or(BridgeError::MissingArg("pattern"))?;
        match glob::glob(pattern) {
            Ok(paths) => Ok(paths.filter_map(Result::ok).count().to_string()),
            Err(e) => Err(BridgeError::Resolve(format!("bad glob: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn counts_matching_files() {
        let dir = std::env::temp_dir().join("lmd_count_bridge");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..3 {
            std::fs::write(dir.join(format!("c{i}.cnt")), "x").unwrap();
        }
        let args = DirectiveArgs::parse(&format!("{}/*.cnt", dir.to_str().unwrap()));
        let out = CountBridge.execute(&ctx(), &args).unwrap();
        assert_eq!(out, "3", "got: {out}");
    }

    #[test]
    fn missing_pattern_errors() {
        let err = CountBridge
            .execute(&ctx(), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn count_is_registered() {
        assert!(super::super::default_registry().get("count").is_some());
    }
}
