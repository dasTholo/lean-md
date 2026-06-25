//! `@lean-md` header pre-scan (spec §4.1 header parser).
//! The header is consumed by a line-based scan BEFORE rushdown sees the body,
//! so config feeds `EngineContext` and never appears in rendered output.

use crate::crp_proto::CrpMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Consumer {
    #[default]
    Ai,
    Human,
}

impl Consumer {
    fn parse(s: &str) -> Self {
        match s.trim() {
            "human" => Consumer::Human,
            _ => Consumer::Ai,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellMode {
    #[default]
    Deny,
    Allow,
}

impl ShellMode {
    fn parse(s: &str) -> Self {
        match s.trim() {
            "allow" => ShellMode::Allow,
            _ => ShellMode::Deny,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtensionsMode {
    #[default]
    Deny,
    Allow,
}

impl ExtensionsMode {
    fn parse(s: &str) -> Self {
        match s.trim() {
            "allow" => ExtensionsMode::Allow,
            _ => ExtensionsMode::Deny,
        }
    }
}

/// Parsed `@lean-md` header config (Phase 1 minimal set + Phase 8 `crp`).
#[derive(Debug, Clone)]
pub struct LeanMdHeader {
    pub version: Option<String>,
    pub consumer: Consumer,
    pub shell: ShellMode,
    pub extensions: ExtensionsMode,
    pub crp: CrpMode,
}

// Core CrpMode default = Tdd; the lmd header default MUST be Off (E-3) so a
// document without `crp=` renders byte-identically to the pre-Phase-8 output.
impl Default for LeanMdHeader {
    fn default() -> Self {
        Self {
            version: None,
            consumer: Consumer::default(),
            shell: ShellMode::default(),
            extensions: ExtensionsMode::default(),
            crp: CrpMode::Off,
        }
    }
}

impl LeanMdHeader {
    /// `@call <plugin_tool>` is gated: plugin tools only run with
    /// `@lean-md extensions=allow` (deny-by-default). WASM `@render` is exempt.
    pub fn extensions_allowed(&self) -> bool {
        self.extensions == ExtensionsMode::Allow
    }
}

/// Lenient value parse for the `crp`/`tdd`/`output` header keys. Mirrors the
/// existing `*::parse` style: unknown → Off (renders never abort, §3.2).
fn parse_crp_value(s: &str) -> CrpMode {
    match s.trim().to_lowercase().as_str() {
        "tdd" | "on" | "max" => CrpMode::Tdd,
        "compact" | "standard" => CrpMode::Compact,
        _ => CrpMode::Off, // off | none | unknown
    }
}

/// Line-based pre-scan. If the first non-blank line starts with `@lean-md`,
/// consume that line plus following `key: value` lines up to the first blank
/// line; return the parsed header and the remaining body. Otherwise: default
/// header and the input unchanged.
pub fn parse_header(input: &str) -> (LeanMdHeader, &str) {
    let mut header = LeanMdHeader::default();
    let first = input.lines().next().unwrap_or("");
    if !first.trim_start().starts_with("@lean-md") {
        return (header, input);
    }

    let mut offset = 0usize;
    let mut body_start = input.len();
    for line in input.split_inclusive('\n') {
        let len = line.len();
        let trimmed = line.trim();
        offset += len;
        if trimmed.is_empty() {
            body_start = offset;
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("@lean-md") {
            let v = rest.trim().trim_start_matches('v');
            if !v.is_empty() {
                header.version = Some(v.to_string());
            }
        } else if let Some((k, v)) = trimmed.split_once(':') {
            match k.trim() {
                "consumer" => header.consumer = Consumer::parse(v),
                "shell" => header.shell = ShellMode::parse(v),
                "extensions" => header.extensions = ExtensionsMode::parse(v),
                "crp" | "tdd" | "output" => header.crp = parse_crp_value(v),
                _ => {}
            }
        }
    }
    (header, &input[body_start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_strips_it() {
        let input = "@lean-md 0.1\nconsumer: ai\nshell: deny\n\n# Body\n";
        let (h, body) = parse_header(input);
        assert_eq!(h.version.as_deref(), Some("0.1"));
        assert_eq!(h.consumer, Consumer::Ai);
        assert_eq!(h.shell, ShellMode::Deny);
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn no_header_returns_full_input_and_defaults() {
        let input = "# Just markdown\n";
        let (h, body) = parse_header(input);
        assert_eq!(h.version, None);
        assert_eq!(h.consumer, Consumer::Ai);
        assert_eq!(body, input);
    }

    #[test]
    fn consumer_human_and_shell_allow_parse() {
        let (h, _) = parse_header("@lean-md\nconsumer: human\nshell: allow\n\nx\n");
        assert_eq!(h.consumer, Consumer::Human);
        assert_eq!(h.shell, ShellMode::Allow);
    }

    #[test]
    fn extensions_allow_parses_and_defaults_deny() {
        let (h, _) = parse_header("@lean-md\nextensions: allow\n\nx\n");
        assert_eq!(h.extensions, ExtensionsMode::Allow);
        assert!(h.extensions_allowed());

        let (d, _) = parse_header("@lean-md\nconsumer: ai\n\nx\n");
        assert_eq!(d.extensions, ExtensionsMode::Deny);
        assert!(
            !d.extensions_allowed(),
            "extensions must be deny-by-default"
        );
    }

    #[test]
    fn parses_crp_canonical_and_aliases() {
        use crate::crp_proto::CrpMode;
        let (h, _) = parse_header("@lean-md\ncrp: tdd\n\nx\n");
        assert_eq!(h.crp, CrpMode::Tdd);
        let (h, _) = parse_header("@lean-md\ncrp: compact\n\nx\n");
        assert_eq!(h.crp, CrpMode::Compact);
        let (h, _) = parse_header("@lean-md\ncrp: off\n\nx\n");
        assert_eq!(h.crp, CrpMode::Off);
        // Aliases: key `tdd` / `output`, value aliases on/max/standard.
        let (h, _) = parse_header("@lean-md\ntdd: on\n\nx\n");
        assert_eq!(h.crp, CrpMode::Tdd);
        let (h, _) = parse_header("@lean-md\noutput: standard\n\nx\n");
        assert_eq!(h.crp, CrpMode::Compact);
    }

    #[test]
    fn crp_unknown_is_lenient_off_and_default_is_off() {
        use crate::crp_proto::CrpMode;
        let (h, _) = parse_header("@lean-md\ncrp: bogus\n\nx\n");
        assert_eq!(h.crp, CrpMode::Off, "unknown crp must fall back to Off");
        // E-3 default guarantee: no crp key → Off.
        let (h, _) = parse_header("@lean-md\nconsumer: ai\n\nx\n");
        assert_eq!(h.crp, CrpMode::Off);
        assert_eq!(LeanMdHeader::default().crp, CrpMode::Off);
    }
}
