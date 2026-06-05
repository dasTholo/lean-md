//! Argument parsing for lmd directives: positional tokens + `key=value` pairs.

/// Parsed arguments of a directive: whitespace-separated tokens, where a token
/// containing `=` becomes a `key=value` pair and everything else is positional.
/// Phase 1 has no quoting — paths/values must not contain spaces (noted limitation).
#[derive(Debug, Default, Clone)]
pub struct DirectiveArgs {
    positional: Vec<String>,
    named: Vec<(String, String)>,
    raw: String,
}

impl DirectiveArgs {
    pub fn parse(raw: &str) -> Self {
        let mut positional = Vec::new();
        let mut named = Vec::new();
        for tok in raw.split_whitespace() {
            match tok.split_once('=') {
                Some((k, v)) if !k.is_empty() => named.push((k.to_string(), v.to_string())),
                _ => positional.push(tok.to_string()),
            }
        }
        Self {
            positional,
            named,
            raw: raw.trim().to_string(),
        }
    }

    pub fn positional(&self, i: usize) -> Option<&str> {
        self.positional.get(i).map(String::as_str)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.named
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The trimmed raw argument string (everything after the directive name),
    /// space-preserving — needed by `@query` whose command contains spaces.
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_positional_and_named() {
        let a = DirectiveArgs::parse("path/to/file.rs mode=signatures");
        assert_eq!(a.positional(0), Some("path/to/file.rs"));
        assert_eq!(a.get("mode"), Some("signatures"));
        assert_eq!(a.get("missing"), None);
        assert_eq!(a.positional(1), None);
    }

    #[test]
    fn empty_is_empty() {
        let a = DirectiveArgs::parse("   ");
        assert_eq!(a.positional(0), None);
        assert_eq!(a.get("x"), None);
    }

    #[test]
    fn raw_preserves_full_command_line() {
        let a = DirectiveArgs::parse("echo hello world mode=x");
        assert_eq!(a.raw(), "echo hello world mode=x");
    }
}
