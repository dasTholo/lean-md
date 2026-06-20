//! `@symbol` Router bridge → read-only code navigation/structure over the
//! lean-ctx `ctx_refactor` LSP/IDE actions (spec §4.2). Six ops:
//! refs/def/impl/declaration/type-hierarchy/overview. The token-bearing
//! property (spec §4.2) is cache-name enrichment: ctx_refactor returns bare
//! `file:line:col`; this bridge reads the target line from the shared
//! EngineContext cache (warm ~13-tok hit, §3.4) and appends the extracted
//! type/symbol name — in Rust, zero agent-context tokens. `declaration` and
//! `type-hierarchy` are IDE-only (§3.3): the BACKEND_REQUIRED envelope passes
//! through unchanged.

use std::collections::HashMap;
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::lmd::args::DirectiveArgs;
use crate::lmd::engine::EngineContext;

pub struct SymbolBridge;

/// Map a user-facing op alias to the ctx_refactor action name. None = unknown.
fn map_op(op: &str) -> Option<&'static str> {
    Some(match op {
        "refs" | "references" => "references",
        "def" | "definition" => "definition",
        "impl" | "implementations" => "implementations",
        "declaration" => "declaration",
        "type-hierarchy" | "type_hierarchy" => "type_hierarchy",
        "overview" | "symbols_overview" => "symbols_overview",
        _ => return None,
    })
}

impl DirectiveBridge for SymbolBridge {
    fn name(&self) -> &'static str {
        "symbol"
    }

    fn execute(
        &self,
        ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
        let action = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @symbol op '{op}'. Use: refs|def|impl|declaration|type-hierarchy|overview"
            ))
        })?;
        let root = ctx.jail_root.to_str().unwrap_or(".");

        if action == "symbols_overview" {
            return overview(ctx, args, root);
        }
        nav(ctx, args, action, root)
    }
}

/// Position-based nav ops: references/definition/implementations/declaration/
/// type_hierarchy. Builds the ctx_refactor arg map from path + line(1-idx) +
/// column(0-idx, default 0) and passes the jail-resolved abs_path. `scope`
/// (refs/impl) and `direction` (type_hierarchy) are forwarded when present.
/// The returned location list is cache-name enriched (see `enrich_locations`);
/// `type_hierarchy` renders its own tree and is left verbatim.
// NOTE: nav does NOT use bridges::addressing::build_target — nav resolves
// name= locally to a position (column_of) and accepts a positional path;
// build_target passes name_path through to the backend. See addressing.rs.
fn nav(
    ctx: &Rc<EngineContext>,
    args: &DirectiveArgs,
    action: &str,
    root: &str,
) -> Result<String, BridgeError> {
    // Symbol addressing: `name=Class/method` resolves to path+position via the
    // shared symbol index (spec §4.2 Z. 265–266). Resolution errors
    // (NO_SYMBOL/AMBIGUOUS_SYMBOL) are surfaced verbatim.
    let (rel_path, line, column) = if let Some(name_path) = args.get("name") {
        match crate::tools::ctx_refactor::resolve_name_path(name_path, root) {
            Ok(r) => {
                let leaf = name_path.rsplit('/').next().unwrap_or(name_path);
                let column = column_of(ctx, root, &r.rel_path, r.start_line, leaf);
                (r.rel_path, r.start_line as u64, column)
            }
            Err(e) => return Ok(format!("ERROR: {e}")),
        }
    } else {
        let path = args
            .positional(1)
            .or_else(|| args.get("path"))
            .ok_or(BridgeError::MissingArg("path"))?;
        let line = args
            .get("line")
            .ok_or(BridgeError::MissingArg("line"))?
            .parse()
            .map_err(|_| BridgeError::Resolve("line must be a 1-based integer".into()))?;
        let column = args.get("column").and_then(|c| c.parse().ok()).unwrap_or(0);
        (path.to_string(), line, column)
    };

    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, &rel_path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;

    let mut obj = serde_json::Map::new();
    obj.insert("action".into(), action.into());
    obj.insert("line".into(), line.into());
    obj.insert("column".into(), column.into());
    if let Some(scope) = args.get("scope") {
        obj.insert("scope".into(), scope.into());
    }
    if let Some(direction) = args.get("direction") {
        obj.insert("direction".into(), direction.into());
    }

    let out = crate::tools::ctx_refactor::handle(&serde_json::Value::Object(obj), root, &abs);
    // Cache-name enrichment is meaningful for location lists (refs/def/impl/
    // declaration). type_hierarchy renders its own tree, so leave it verbatim.
    if action == "type_hierarchy" {
        Ok(out)
    } else {
        Ok(enrich_locations(&out, ctx, root))
    }
}

/// `@symbol overview <path>` → ctx_refactor symbols_overview (path only).
fn overview(
    _ctx: &Rc<EngineContext>,
    args: &DirectiveArgs,
    root: &str,
) -> Result<String, BridgeError> {
    let path = args
        .positional(1)
        .or_else(|| args.get("path"))
        .ok_or(BridgeError::MissingArg("path"))?;
    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, path)
        .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;
    let obj = serde_json::json!({ "action": "symbols_overview" });
    Ok(crate::tools::ctx_refactor::handle(&obj, root, &abs))
}

/// `impl X for Y { … }` → `Y`; `impl Y { … }` → `Y`; otherwise None.
/// Generics are skipped: `impl<T> Tr<T> for Wid<T>` → `Wid` (spec §4.2 Z. 262).
fn extract_impl_type(line: &str) -> Option<String> {
    // Strip the `impl` keyword, tolerating a generic param list with no space:
    // `impl Bar` and `impl<T> Tr<T> for Wid<T>` both reduce to the post-`impl`
    // remainder.
    let after = line.trim_start().strip_prefix("impl")?;
    if !after.starts_with([' ', '<']) {
        return None;
    }
    // Skip a leading `impl<…>` generic param list (balanced angle brackets) so
    // `impl<T> Tr<T> for Wid<T>` re-anchors on the trait/type token.
    let rest = skip_generics(after.trim_start());
    let target = match rest.find(" for ") {
        Some(i) => &rest[i + " for ".len()..],
        None => rest,
    };
    let name: String = target
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
        .collect();
    let name = name.trim_end_matches(':').to_string();
    if name.is_empty() { None } else { Some(name) }
}

/// If `s` starts with a `<…>` generic param block, return the remainder after
/// the balanced closing `>` (whitespace-trimmed); otherwise return `s` as-is.
fn skip_generics(s: &str) -> &str {
    if !s.starts_with('<') {
        return s;
    }
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return s[i + c.len_utf8()..].trim_start();
                }
            }
            _ => {}
        }
    }
    s
}

/// Annotation for one location's source line: the impl type name if present,
/// else the trimmed line (truncated) so the agent sees the code without a
/// re-read (the §4.2 token win).
fn annotate_line(line_text: &str) -> String {
    if let Some(name) = extract_impl_type(line_text) {
        return name;
    }
    let t = line_text.trim();
    let snip: String = t.chars().take(80).collect();
    if t.chars().count() > 80 {
        format!("{snip}…")
    } else {
        snip
    }
}

/// Parse a `  {rel_path}:{line}:{col}` location line into (rel_path, 1-based
/// line). Header/note/"Total:" lines (no trailing `:int:int`) return None.
fn parse_location_line(line: &str) -> Option<(String, usize)> {
    let t = line.trim();
    let (rest, col) = t.rsplit_once(':')?;
    col.parse::<usize>().ok()?;
    let (path, ln) = rest.rsplit_once(':')?;
    let ln: usize = ln.parse().ok()?;
    if path.is_empty() {
        return None;
    }
    Some((path.to_string(), ln))
}

/// Cache-name enrichment (spec §4.2 Z. 259–266): for each `file:line:col`
/// location, read the target line from the shared EngineContext cache (warm
/// ~13-tok hit, §3.4) and append the extracted type/symbol name. Non-location
/// lines pass through unchanged.
fn enrich_locations(raw: &str, ctx: &Rc<EngineContext>, root: &str) -> String {
    // Per-call content cache: decompress each distinct file at most once. N hits
    // in the same file would otherwise re-decompress the whole file N× (each
    // `line_from_cache` does a full `get_full_content`). The map lives only for
    // the duration of THIS call (one location list) so it cannot serve a stale
    // read across invocations — mtime cache coherence is unaffected.
    let mut contents: HashMap<String, String> = HashMap::new();
    let mut out = String::new();
    for line in raw.lines() {
        if let Some((rel, ln)) = parse_location_line(line) {
            let content = contents
                .entry(rel.clone())
                .or_insert_with(|| file_content_from_cache(ctx, root, &rel).unwrap_or_default());
            if let Some(text) = content.lines().nth(ln.saturating_sub(1)) {
                out.push_str(line);
                out.push_str("  → ");
                out.push_str(&annotate_line(text));
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// 0-based column of `leaf` on line `ln` (1-based) of `rel`, read from the
/// shared cache; 0 if not found (the position still lands on the symbol's line).
/// Prefers a word-boundary match so a leaf that also occurs as a substring of an
/// earlier identifier (`Bar` inside `FooBar`) does not steal the column; falls
/// back to the first substring occurrence, then to column 0.
fn column_of(ctx: &Rc<EngineContext>, root: &str, rel: &str, ln: usize, leaf: &str) -> u64 {
    let Some(text) = line_from_cache(ctx, root, rel, ln) else {
        return 0;
    };
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let byte = text
        .match_indices(leaf)
        .find(|(i, m)| {
            let before_ok = text[..*i].chars().next_back().is_none_or(|c| !is_ident(c));
            let after_ok = text[i + m.len()..]
                .chars()
                .next()
                .is_none_or(|c| !is_ident(c));
            before_ok && after_ok
        })
        .map(|(i, _)| i)
        .or_else(|| text.find(leaf));
    byte.map_or(0, |b| text[..b].chars().count() as u64)
}

/// Read line `ln` (1-based) of `rel` from the shared cache; warm it first via a
/// `ctx_read` full read so the lookup is a ~13-tok cache hit (spec §3.4). Never
/// `fresh`/`raw`.
fn line_from_cache(ctx: &Rc<EngineContext>, root: &str, rel: &str, ln: usize) -> Option<String> {
    file_content_from_cache(ctx, root, rel)?
        .lines()
        .nth(ln.saturating_sub(1))
        .map(str::to_string)
}

/// Read the FULL content of `rel` from the shared cache (one decompress),
/// warming it first via a `ctx_read` full read so the lookup is a ~13-tok cache
/// hit (spec §3.4). Never `fresh`/`raw`. Callers that need many lines of the
/// same file index into this string once instead of re-decompressing per line.
fn file_content_from_cache(ctx: &Rc<EngineContext>, root: &str, rel: &str) -> Option<String> {
    let abs = crate::core::path_resolve::resolve_tool_path(Some(root), None, rel).ok()?;
    {
        let mut cache = ctx.cache.borrow_mut();
        let _ =
            crate::tools::ctx_read::handle(&mut cache, &abs, "full", crate::tools::CrpMode::Off);
    }
    ctx.cache.borrow().get_full_content(&abs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmd::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx_at(root: PathBuf) -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), root))
    }

    #[test]
    fn op_aliases_map_to_ctx_refactor_actions() {
        assert_eq!(map_op("refs"), Some("references"));
        assert_eq!(map_op("def"), Some("definition"));
        assert_eq!(map_op("impl"), Some("implementations"));
        assert_eq!(map_op("declaration"), Some("declaration"));
        assert_eq!(map_op("type-hierarchy"), Some("type_hierarchy"));
        assert_eq!(map_op("overview"), Some("symbols_overview"));
        assert_eq!(map_op("frobnicate"), None);
    }

    #[test]
    fn unknown_op_is_a_clear_error() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate x.rs"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("unknown @symbol op"), "got: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }

    #[test]
    fn missing_op_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("op")), "got: {err:?}");
    }

    #[test]
    fn overview_routes_on_a_real_rust_file() {
        // Headless tree-sitter symbols_overview on a real symbol — never a panic.
        let dir = std::env::temp_dir().join("lmd_symbol_overview");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("o.rs");
        std::fs::write(&f, "pub fn alpha() {}\npub struct Beta;\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("overview {}", f.to_str().unwrap()));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        // Either an overview listing or a precise degradation envelope.
        assert!(!out.trim().is_empty(), "overview produced empty output");
        assert!(
            out.contains("alpha")
                || out.contains("Beta")
                || out.contains("ERROR")
                || out.contains("No results")
                || out.contains("No symbols"),
            "unexpected overview output: {out}"
        );
    }

    #[test]
    fn overview_missing_path_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("overview"))
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("path")),
            "got: {err:?}"
        );
    }

    #[test]
    fn symbol_is_registered() {
        assert!(super::super::default_registry().get("symbol").is_some());
    }

    #[test]
    fn impl_op_requires_a_position() {
        let dir = std::env::temp_dir();
        let ctx = ctx_at(dir.clone());
        let f = dir.join("pos_required.rs");
        // refs/def/impl/declaration/type-hierarchy without line= and without
        // name= cannot resolve a position → clear error. The MissingArg("line")
        // check fires before the jail-resolve, so the file need not exist; use
        // an absolute temp path so the test is CWD-independent (matches the
        // other dispatch tests in this module).
        let err = SymbolBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse(&format!("impl {}", f.to_str().unwrap())),
            )
            .unwrap_err();
        assert!(
            matches!(err, BridgeError::MissingArg("line")),
            "got: {err:?}"
        );
    }

    #[test]
    fn refs_op_routes_with_position() {
        // A real position; headless rust-analyzer or a degradation envelope —
        // never a panic, and never the unknown-op error.
        let dir = std::env::temp_dir().join("lmd_symbol_refs");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn helper() {}\nfn caller() { helper(); }\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("refs {} line=1 column=3", f.to_str().unwrap()));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "refs produced empty output");
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
    }

    #[test]
    fn declaration_is_ide_only_degradation_passes_through() {
        // declaration/type-hierarchy are IDE-only (spec §3.3). Without a running
        // IDE the bridge must surface ctx_refactor's degradation envelope
        // verbatim, not crash and not invent a result.
        let dir = std::env::temp_dir().join("lmd_symbol_decl");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("d.rs");
        std::fs::write(&f, "pub struct Widget;\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!(
            "declaration {} line=1 column=11",
            f.to_str().unwrap()
        ));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(!out.trim().is_empty(), "declaration produced empty output");
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
    }

    #[test]
    fn extract_impl_type_handles_for_and_inherent() {
        // spec §4.2: "impl X for Y -> Y"
        assert_eq!(
            super::extract_impl_type("impl Foo for Bar {"),
            Some("Bar".into())
        );
        assert_eq!(
            super::extract_impl_type("    impl Bar {"),
            Some("Bar".into())
        );
        assert_eq!(
            super::extract_impl_type("impl<T> Trait<T> for Widget<T> {"),
            Some("Widget".into())
        );
        assert_eq!(super::extract_impl_type("fn helper() {}"), None);
    }

    #[test]
    fn annotate_line_falls_back_to_trimmed_snippet() {
        assert_eq!(super::annotate_line("impl Foo for Bar {"), "Bar");
        assert_eq!(
            super::annotate_line("    fn helper(x: u32) {}"),
            "fn helper(x: u32) {}"
        );
    }

    #[test]
    fn parse_location_line_extracts_path_and_line() {
        assert_eq!(
            super::parse_location_line("  src/a.rs:42:7"),
            Some(("src/a.rs".into(), 42usize))
        );
        // Header / notes / "Total:" lines are not locations.
        assert_eq!(super::parse_location_line("3 location(s):"), None);
        assert_eq!(super::parse_location_line("No results found."), None);
    }

    #[test]
    fn enrich_appends_impl_type_name_from_cache() {
        // A synthetic ctx_refactor-style location list against a real fixture,
        // proving enrichment reads the target line and appends the type name —
        // independent of a live language server.
        let dir = std::env::temp_dir().join("lmd_symbol_enrich");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("impls.rs");
        std::fs::write(
            &f,
            "struct Bar;\ntrait Foo {}\nimpl Foo for Bar {\n    fn m(&self) {}\n}\n",
        )
        .unwrap();
        let ctx = ctx_at(dir.clone());

        // Location line points at the `impl Foo for Bar` line (1-based L3).
        let raw = format!("1 location(s):\n  {}:3:0\n", f.to_str().unwrap());
        let out = super::enrich_locations(&raw, &ctx, dir.to_str().unwrap());
        assert!(out.contains(":3:0"), "original location must remain: {out}");
        assert!(
            out.contains("Bar"),
            "enrichment must append the impl type: {out}"
        );
    }

    #[test]
    fn enrich_passes_through_non_location_lines() {
        let ctx = ctx_at(PathBuf::from("."));
        let raw = "No results found.";
        let out = super::enrich_locations(raw, &ctx, ".");
        assert_eq!(out.trim(), "No results found.");
    }

    #[test]
    fn name_addressing_resolves_position_for_a_real_symbol() {
        // `name=` must resolve via resolve_name_path → dispatch the op without
        // requiring an explicit line=. Headless: a result list or a degradation
        // envelope, never the missing-line error and never a panic.
        let dir = std::env::temp_dir().join("lmd_symbol_name_addr");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("n.rs");
        std::fs::write(&f, "pub fn uniquely_named_fn() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse("impl name=uniquely_named_fn");
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(
            !out.contains("MissingArg"),
            "name= must supply the position: {out}"
        );
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
        assert!(!out.trim().is_empty(), "empty output");
    }

    #[test]
    fn name_addressing_unknown_symbol_is_clear() {
        let ctx = ctx_at(PathBuf::from("."));
        let out = SymbolBridge
            .execute(
                &ctx,
                &DirectiveArgs::parse("impl name=ThisSymbolDoesNotExist_xyz"),
            )
            .unwrap();
        // resolve_name_path returns a NO_SYMBOL error string; surface it.
        assert!(
            out.contains("NO_SYMBOL") || out.contains("no symbol"),
            "unknown name must produce a clear no-symbol message: {out}"
        );
    }

    #[test]
    fn type_hierarchy_passes_direction() {
        let dir = std::env::temp_dir().join("lmd_symbol_th");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.rs");
        std::fs::write(&f, "trait Animal {}\nstruct Dog;\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!(
            "type-hierarchy {} line=2 column=7 direction=subtypes",
            f.to_str().unwrap()
        ));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(
            !out.trim().is_empty(),
            "type-hierarchy produced empty output"
        );
        assert!(
            !out.contains("unknown @symbol op"),
            "op must dispatch: {out}"
        );
    }

    #[test]
    fn column_of_prefers_word_boundary_over_substring() {
        // `Bar` occurs inside `FooBar` (byte 8) and standalone (byte 16); the
        // word-boundary match must pick the standalone occurrence so name=
        // addressing lands on the right symbol, not an earlier substring.
        let dir = std::env::temp_dir().join("lmd_symbol_colof");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("c.rs");
        std::fs::write(&f, "impl FooBar for Bar {\n}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let col = super::column_of(&ctx, dir.to_str().unwrap(), f.to_str().unwrap(), 1, "Bar");
        assert_eq!(
            col, 16,
            "must pick standalone Bar, not the FooBar substring"
        );
    }
}
