//! Phase-9 human-readable gloss: directive name+args → German prose.
//! The table is embedded from `lean-md/gloss/directives.lmd.md` via
//! `include_str!` (compile-time, byte-stable #498). Only Work-directives
//! (render::WORK_DIRECTIVES) are glossed; everything else renders normally.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::args::DirectiveArgs;

const GLOSS_TABLE_SRC: &str = include_str!("../../../lean-md/gloss/directives.lmd.md");

fn table() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| parse_table(GLOSS_TABLE_SRC))
}

/// Parse the embedded markdown table into a `key → template` map. Skips the
/// header row, the `---` separator row and any non-table line.
pub(crate) fn parse_table(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in src.lines() {
        let t = line.trim();
        if !t.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = t.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 {
            continue;
        }
        let key = cells[0];
        if key.is_empty()
            || key.eq_ignore_ascii_case("direktive")
            || key.chars().all(|c| c == '-' || c == ':')
        {
            continue;
        }
        map.insert(key.to_string(), cells[1].to_string());
    }
    map
}

/// Resolve the lookup key for a directive: `name:op` first (op = positional 0,
/// e.g. `@symbol refs`, or the first named key, e.g. `@graph dependents=…`),
/// then the bare `name`, then a generic fallback.
fn render_template(name: &str, args: &DirectiveArgs) -> String {
    let op_key = args
        .positional(0)
        .map(|p| format!("{name}:{p}"))
        .or_else(|| {
            args.named_pairs()
                .first()
                .map(|(k, _)| format!("{name}:{k}"))
        });
    let tmpl = op_key
        .as_deref()
        .and_then(|k| table().get(k))
        .or_else(|| table().get(name));
    match tmpl {
        Some(t) => substitute(t, args),
        None => format!("Direktive `@{name}`: `{}`", args.raw().trim()),
    }
}

/// Substitute `{slot}` placeholders: `{N}`→positional N, `{raw}`→full args,
/// `{key}`→named arg. Missing slots resolve to empty.
fn substitute(tmpl: &str, args: &DirectiveArgs) -> String {
    let mut out = String::with_capacity(tmpl.len());
    let mut rest = tmpl;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        if let Some(close) = rest[open..].find('}') {
            let key = &rest[open + 1..open + close];
            out.push_str(&resolve_slot(key, args));
            rest = &rest[open + close + 1..];
        } else {
            out.push_str(&rest[open..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

fn resolve_slot(key: &str, args: &DirectiveArgs) -> String {
    if key == "raw" {
        return args.tokens_joined();
    }
    if let Ok(n) = key.parse::<usize>() {
        return args.positional(n).unwrap_or("").to_string();
    }
    args.get(key).unwrap_or("").to_string()
}

/// Human-readable gloss for a Work-directive. Public entry used by the
/// `consumer=human` render branch in `render::dispatch`.
pub(crate) fn gloss(name: &str, raw_args: &str) -> String {
    let args = DirectiveArgs::parse(raw_args);
    render_template(name, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glosses_common_work_directives() {
        assert_eq!(
            gloss("read", "src/parser/block.rs"),
            "Datei `src/parser/block.rs` lesen"
        );
        assert_eq!(
            gloss("query", "\"cargo nextest run\""),
            "Ausführen: `cargo nextest run`"
        );
        assert_eq!(
            gloss("graph", "dependents=parse_block"),
            "Abhängige von `parse_block` ermitteln"
        );
        assert_eq!(
            gloss("symbol", "refs parse_block"),
            "Referenzen von `parse_block` ermitteln"
        );
    }

    #[test]
    fn unknown_directive_uses_generic_fallback() {
        assert_eq!(gloss("frobnicate", "x y"), "Direktive `@frobnicate`: `x y`");
    }

    #[test]
    fn table_parses_nonempty_and_skips_header() {
        let t = table();
        assert!(t.contains_key("read"), "read entry present");
        assert!(!t.contains_key("Direktive"), "header row skipped");
    }

    #[test]
    fn embedded_table_matches_on_disk_file() {
        // include_str! identity (Spec §6.4): the embedded bytes are the file.
        let on_disk = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../lean-md/gloss/directives.lmd.md"
        ))
        .expect("gloss file readable");
        assert_eq!(
            GLOSS_TABLE_SRC, on_disk,
            "embedded gloss drifted from on-disk file"
        );
    }
}
