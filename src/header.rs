//! `@lean-md` header pre-scan (spec §4.1 header parser).
//! The header is consumed by a line-based scan BEFORE rushdown sees the body,
//! so config feeds `EngineContext` and never appears in rendered output.

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

/// Parsed `@lean-md` header config (Phase 1 minimal set).
#[derive(Debug, Clone, Default)]
pub struct LeanMdHeader {
    pub version: Option<String>,
    pub consumer: Consumer,
    pub shell: ShellMode,
    pub extensions: ExtensionsMode,
}

impl LeanMdHeader {
    /// `@call <plugin_tool>` is gated: plugin tools only run with
    /// `@lean-md extensions=allow` (deny-by-default). WASM `@render` is exempt.
    pub fn extensions_allowed(&self) -> bool {
        self.extensions == ExtensionsMode::Allow
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
}
