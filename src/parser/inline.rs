//! Inline parser: claims `{{ name args }}` into an `LmdInline` node.
//! Mirrors the Phase-0 spike's ShoutInline (trigger `{`, manual advance of the
//! full match because the inline dispatcher does not pre-consume the trigger).

use rushdown::ast::*;
use rushdown::parser;
use rushdown::parser::*;
use rushdown::text;
use rushdown::text::Reader;

use super::super::node::LmdInline;

/// True if `name` matches the lmd directive-name grammar: an ascii-alphabetic
/// first byte, then only `[a-z0-9-]` (ascii-alphanumeric or `-`). Mirrors the
/// block grammar in `block.rs::parse_directive_line` so inline and block
/// directives share one charset (spec §9 F-2).
fn is_valid_directive_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    match bytes.first() {
        Some(b) if b.is_ascii_alphabetic() => {}
        _ => return false,
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

/// Pure recognizer for the body between `{{ ` and ` }}`: first token is the
/// directive name, the remainder (trimmed) is the args. Returns None if empty
/// or if the directive name does not match the `[a-z0-9-]`-with-alpha-start
/// charset (spec §9 F-2).
pub fn parse_inline_body(body: &str) -> Option<(String, String)> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    let (name, args) = match body.split_once(char::is_whitespace) {
        Some((name, args)) => (name.to_string(), args.trim().to_string()),
        None => (body.to_string(), String::new()),
    };
    if !is_valid_directive_name(&name) {
        return None;
    }
    Some((name, args))
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

    #[test]
    fn rejects_comment_injection_name() {
        // F-2: a name that is not [a-z0-9-]-with-alpha-start must NOT be claimed,
        // so `{{ -->x }}` can never reach the HTML-comment render fallback.
        assert!(parse_inline_body("-->x").is_none());
        assert!(parse_inline_body("a-->b").is_none());
        assert!(parse_inline_body("<script").is_none());
        // valid names still parse
        assert_eq!(parse_inline_body("read").unwrap().0, "read");
        assert_eq!(parse_inline_body("hard-rules x").unwrap().0, "hard-rules");
    }
}
