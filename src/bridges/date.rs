//! `@date` Router bridge → current local date via chrono (Spec §3.1 trivial-R).
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

/// `@date [fmt=<strftime>]` — current local date, default `%Y-%m-%d`.
pub struct DateBridge;

impl DirectiveBridge for DateBridge {
    fn name(&self) -> &'static str {
        "date"
    }

    fn execute(
        &self,
        _ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let fmt = args.get("fmt").unwrap_or("%Y-%m-%d");
        Ok(chrono::Local::now().format(fmt).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::args::DirectiveArgs;
    use crate::lmd::engine::EngineContext;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), PathBuf::from(".")))
    }

    #[test]
    fn default_format_is_iso_date() {
        let out = DateBridge.execute(&ctx(), &DirectiveArgs::parse("")).unwrap();
        // YYYY-MM-DD => length 10, dashes at index 4 and 7.
        assert_eq!(out.len(), 10, "got: {out}");
        assert_eq!(out.as_bytes()[4], b'-', "got: {out}");
        assert_eq!(out.as_bytes()[7], b'-', "got: {out}");
    }

    #[test]
    fn custom_format_year_only() {
        let out = DateBridge
            .execute(&ctx(), &DirectiveArgs::parse("fmt=%Y"))
            .unwrap();
        assert_eq!(out.len(), 4, "got: {out}");
        assert!(out.chars().all(|c| c.is_ascii_digit()), "got: {out}");
    }

    #[test]
    fn date_is_registered() {
        assert!(super::super::default_registry().get("date").is_some());
    }
}
