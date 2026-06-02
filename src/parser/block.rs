//! Block parser: claims `@<name> [args]` lines into an `LmdDirective` node.
//! Stateless (like the Phase-0 spike); bridge dispatch happens at render time.

use rushdown::ast::*;
use rushdown::parser;
use rushdown::parser::*;
use rushdown::text;
use rushdown::text::Reader;

use super::super::node::LmdDirective;

/// Pure syntax recognizer: `@<ident> [args]\n` -> (name, args).
/// `ident` = ascii-alpha start, then `[a-z0-9-]`. Returns None for non-matches
/// so normal text/`@`-lines that aren't directives pass through untouched.
pub fn parse_directive_line(line: &[u8]) -> Option<(String, String)> {
    let rest = line.strip_prefix(b"@")?;
    let first = *rest.first()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let name_len = rest
        .iter()
        .position(|b| !(b.is_ascii_alphanumeric() || *b == b'-'))
        .unwrap_or(rest.len());
    if name_len == 0 {
        return None;
    }
    let name = String::from_utf8_lossy(&rest[..name_len]).to_string();
    let args_raw = &rest[name_len..];
    let args = String::from_utf8_lossy(args_raw)
        .trim_matches(|c: char| c.is_whitespace())
        .to_string();
    Some((name, args))
}

#[derive(Debug, Default)]
pub struct LmdBlockParser {}

impl LmdBlockParser {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlockParser for LmdBlockParser {
    fn trigger(&self) -> &[u8] {
        b"@"
    }

    fn open(
        &self,
        arena: &mut Arena,
        _parent_ref: NodeRef,
        reader: &mut text::BasicReader,
        _ctx: &mut parser::Context,
    ) -> Option<(NodeRef, State)> {
        let (line, _seg) = reader.peek_line_bytes()?;
        let (name, args) = parse_directive_line(line.as_ref())?;
        reader.advance_to_eol();
        Some((
            arena.new_node(LmdDirective::new(name, args)),
            State::NO_CHILDREN,
        ))
    }

    fn cont(
        &self,
        _arena: &mut Arena,
        _node_ref: NodeRef,
        _reader: &mut text::BasicReader,
        _ctx: &mut parser::Context,
    ) -> Option<State> {
        None
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
}

impl From<LmdBlockParser> for AnyBlockParser {
    fn from(p: LmdBlockParser) -> Self {
        AnyBlockParser::Extension(Box::new(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_args() {
        let r = parse_directive_line(b"@read path/to/x.rs mode=full\n").unwrap();
        assert_eq!(r.0, "read");
        assert_eq!(r.1, "path/to/x.rs mode=full");
    }

    #[test]
    fn parses_bare_directive() {
        let r = parse_directive_line(b"@include hard-rules\n").unwrap();
        assert_eq!(r.0, "include");
        assert_eq!(r.1, "hard-rules");
    }

    #[test]
    fn rejects_non_directive() {
        assert!(parse_directive_line(b"plain text\n").is_none());
        assert!(parse_directive_line(b"@\n").is_none());
        assert!(parse_directive_line(b"@1bad name\n").is_none());
    }
}
