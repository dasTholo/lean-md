//! Vendored CRP protocol primitives (was `core::protocol`). Self-contained:
//! the lean-md render core decides CRP verbosity in-process; no lean_ctx link.

/// Context Reduction Protocol mode controlling output verbosity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrpMode {
    Off,
    Compact,
    Tdd,
}

impl CrpMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "off" => Some(Self::Off),
            "compact" => Some(Self::Compact),
            "tdd" => Some(Self::Tdd),
            _ => None,
        }
    }
}

/// Whether non-essential meta lines (cache refs, budget warnings, repetition
/// hints) should be shown. Default false; opt-in via env var.
pub fn meta_visible() -> bool {
    if matches!(std::env::var("LEAN_CTX_QUIET"), Ok(v) if v.trim() == "1") {
        return false;
    }
    matches!(std::env::var("LEAN_CTX_META"), Ok(v) if v.trim() == "1")
        || matches!(std::env::var("LEAN_CTX_DIAGNOSTICS"), Ok(v) if v.trim() == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrips_known_modes() {
        assert_eq!(CrpMode::parse("off"), Some(CrpMode::Off));
        assert_eq!(CrpMode::parse(" Tdd "), Some(CrpMode::Tdd));
        assert_eq!(CrpMode::parse("compact"), Some(CrpMode::Compact));
        assert_eq!(CrpMode::parse("nope"), None);
    }
}
