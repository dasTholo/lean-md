//! Argument parsing for lmd directives: positional tokens + `key=value` pairs.

/// Parsed arguments of a directive: whitespace-separated tokens, where a token
/// containing `=` becomes a `key=value` pair and everything else is positional.
/// Quoted values are supported: double "..." (with \n \t \r \" \\ escapes) and single '...' (literal). raw() stays verbatim.
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
        for tok in tokenize(raw) {
            match tok.split_once('=') {
                Some((k, v)) if !k.is_empty() => named.push((k.to_string(), v.to_string())),
                _ => positional.push(tok),
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

/// Whitespace-split that honors `"double"` and `'single'` quotes. A token may
/// mix unquoted and quoted parts (`key="a b"` -> one token `key=a b`). Double
/// quotes decode escapes (`\n \t \r \" \\`); single quotes are literal (shell
/// convention). Unquoted input tokenizes exactly like `split_whitespace`, so
/// every pre-existing directive keeps its behavior. `raw` is untouched.
fn tokenize(raw: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut in_tok = false;
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            if in_tok {
                toks.push(std::mem::take(&mut cur));
                in_tok = false;
            }
            continue;
        }
        in_tok = true;
        match c {
            '"' => {
                while let Some(c2) = chars.next() {
                    match c2 {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => cur.push('\n'),
                            Some('t') => cur.push('\t'),
                            Some('r') => cur.push('\r'),
                            Some('"') => cur.push('"'),
                            Some('\\') | None => cur.push('\\'),
                            Some(other) => {
                                cur.push('\\');
                                cur.push(other);
                            }
                        },
                        _ => cur.push(c2),
                    }
                }
            }
            '\'' => {
                for c2 in chars.by_ref() {
                    if c2 == '\'' {
                        break;
                    }
                    cur.push(c2);
                }
            }
            _ => cur.push(c),
        }
    }
    if in_tok {
        toks.push(cur);
    }
    toks
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

    #[test]
    fn parses_double_quoted_value_with_spaces() {
        let a = DirectiveArgs::parse(r#"path.rs old="foo bar" new="baz qux""#);
        assert_eq!(a.positional(0), Some("path.rs"));
        assert_eq!(a.get("old"), Some("foo bar"));
        assert_eq!(a.get("new"), Some("baz qux"));
    }

    #[test]
    fn decodes_escapes_in_double_quotes() {
        let a = DirectiveArgs::parse(r#"body="line1\nline2\ttab""#);
        assert_eq!(a.get("body"), Some("line1\nline2\ttab"));
    }

    #[test]
    fn single_quotes_are_literal() {
        let a = DirectiveArgs::parse(r"new='a\nb'");
        assert_eq!(a.get("new"), Some(r"a\nb"));
    }

    #[test]
    fn empty_quoted_value_is_preserved() {
        let a = DirectiveArgs::parse(r#"new="""#);
        assert_eq!(a.get("new"), Some(""));
    }

    #[test]
    fn quoted_positional_with_spaces() {
        let a = DirectiveArgs::parse(r#""foo bar" mode=x"#);
        assert_eq!(a.positional(0), Some("foo bar"));
        assert_eq!(a.get("mode"), Some("x"));
    }

    #[test]
    fn raw_is_unchanged_by_quoting() {
        // @query relies on raw() — it must keep the verbatim string incl. quotes.
        let cmd = r#"echo "hello world""#;
        let a = DirectiveArgs::parse(cmd);
        assert_eq!(a.raw(), cmd);
    }
}
