//! lmd rushdown parser extension wiring (block + inline parsers).

pub mod block;
pub mod inline;

use rushdown::parser::{
    NoParserOptions, PRIORITY_ATX_HEADING, PRIORITY_EMPHASIS, ParserExtension, parser_extension,
};

use block::LmdBlockParser;
use inline::LmdInlineParser;

/// Registers both lmd parsers. Block sits at heading priority (claim `@`-lines
/// before they become paragraphs); inline runs after emphasis (the spike's
/// `PRIORITY_EMPHASIS + 100`); `{` is not a CommonMark inline trigger.
pub fn lmd_parser_extension() -> impl ParserExtension {
    parser_extension(|p| {
        p.add_block_parser(LmdBlockParser::new, NoParserOptions, PRIORITY_ATX_HEADING);
        p.add_inline_parser(
            LmdInlineParser::new,
            NoParserOptions,
            PRIORITY_EMPHASIS + 100,
        );
    })
}
