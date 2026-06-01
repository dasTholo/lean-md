//! Inline parser: claims `{{ name args }}` into an `LmdInline` node.
//! Mirrors the Phase-0 spike's ShoutInline (trigger `{`, manual advance of the
//! full match because the inline dispatcher does not pre-consume the trigger).

use rushdown::ast::*;
use rushdown::parser;
use rushdown::parser::*;
use rushdown::text;
use rushdown::text::Reader;

use super::super::node::LmdInline;

/// Pure recognizer for the body between `{{ ` and ` }}`: first token is the
/// directive name, the remainder (trimmed) is the args. Returns None if empty.
pub fn parse_inline_body(body: &str) -> Option<(String, String)> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    match body.split_once(char::is_whitespace) {
        Some((name, args)) => Some((name.to_string(), args.trim().to_string())),
        None => Some((body.to_string(), String::new())),
    }
}

#[derive(Debug, Default)]
pub struct LmdInlineParser {}

impl LmdInlineParser {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InlineParser for LmdInlineParser {
    fn trigger(&self) -> &[u8] {
        b"{"
    }

    fn parse(
        &self,
        arena: &mut Arena,
        _parent_ref: NodeRef,
        reader: &mut text::BlockReader,
        _ctx: &mut parser::Context,
    ) -> Option<NodeRef> {
        let (line, _seg) = reader.peek_line_bytes()?;
        const OPEN: &[u8] = b"{{ ";
        const CLOSE: &[u8] = b" }}";
        if !line.starts_with(OPEN) {
            return None;
        }
        let body_start = OPEN.len();
        let mut i = body_start;
        let close_at = loop {
            if i + CLOSE.len() > line.len() {
                return None;
            }
            if &line[i..i + CLOSE.len()] == CLOSE {
                break i;
            }
            i += 1;
        };
        let body = String::from_utf8_lossy(&line[body_start..close_at]).to_string();
        let (name, args) = parse_inline_body(&body)?;
        let consumed = close_at + CLOSE.len();
        reader.advance(consumed);
        Some(arena.new_node(LmdInline::new(name, args)))
    }
}

impl From<LmdInlineParser> for AnyInlineParser {
    fn from(p: LmdInlineParser) -> Self {
        AnyInlineParser::Extension(Box::new(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_name_and_args() {
        let r = parse_inline_body("include hard-rules").unwrap();
        assert_eq!(r.0, "include");
        assert_eq!(r.1, "hard-rules");
    }

    #[test]
    fn parses_inline_name_only() {
        let r = parse_inline_body("date").unwrap();
        assert_eq!(r.0, "date");
        assert_eq!(r.1, "");
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_inline_body("   ").is_none());
    }
}
