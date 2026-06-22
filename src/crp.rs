//! Phase-8 CRP helpers (lmd-local). Notation rendering + the deterministic
//! guidance block that `apply_crp_hook` appends for `compact`/`tdd`. All data
//! derives from the canonical `core::tdd_schema` single source — this module
//! only READS it (no `tools/`/`core/` edits, spec Phase-8 edit-jail).

use crate::core::protocol::CrpMode;
use crate::core::signatures::{Signature, extract_signatures};

/// Output rules pulled from the canonical tdd-schema (`crp.output_rules`).
/// `Off` → none. `Compact`/`Tdd` share the schema's single rule list.
pub fn crp_output_rules(mode: CrpMode) -> Vec<String> {
    if mode == CrpMode::Off {
        return Vec::new();
    }
    let schema = crate::core::tdd_schema::tdd_schema_value();
    schema["crp"]["output_rules"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Byte-stable guidance suffix block (#498): a fixed `<!-- crp:MODE -->` header
/// followed by the output rules as bullets. Pure function of `mode`.
pub fn crp_guidance_block(mode: CrpMode) -> String {
    let header = match mode {
        CrpMode::Off => return String::new(),
        CrpMode::Compact => "<!-- crp:compact -->",
        CrpMode::Tdd => "<!-- crp:tdd -->",
    };
    let mut s = String::new();
    s.push_str(header);
    s.push('\n');
    for rule in crp_output_rules(mode) {
        s.push_str("- ");
        s.push_str(&rule);
        s.push('\n');
    }
    s
}

/// Phase-9 human-readable counterpart of `core::signatures::tdd_legend`:
/// the SAME kind buckets, expanded to German words (no dense glyphs). Used by
/// the `consumer=human` branch of `apply_crp_hook` (D-12). Pure function.
pub fn human_legend<'a>(sigs: &[&'a Signature]) -> String {
    if sigs.is_empty() {
        return String::new();
    }
    let mut parts: Vec<&str> = Vec::new();
    let has = |pred: &dyn Fn(&'a Signature) -> bool| sigs.iter().any(|s| pred(s));
    if has(&|s| matches!(s.kind, "fn" | "method")) {
        parts.push("Funktion");
    }
    if has(&|s| matches!(s.kind, "class" | "struct")) {
        parts.push("Klasse/Struct");
    }
    if has(&|s| matches!(s.kind, "interface" | "trait")) {
        parts.push("Trait/Interface");
    }
    if has(&|s| s.kind == "type") {
        parts.push("Typ");
    }
    if has(&|s| s.kind == "enum") {
        parts.push("Enum");
    }
    if has(&|s| matches!(s.kind, "const" | "let" | "var")) {
        parts.push("Wert/Konstante");
    }
    if has(&|s| s.is_exported) {
        parts.push("öffentlich");
    }
    if has(&|s| s.is_async) {
        parts.push("asynchron");
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("**Verwendete Notation:** {}", parts.join(", "))
    }
}

/// Render a file's signatures in the given CRP mode WITHOUT a legend (the
/// End-Hook owns the aggregated legend, E-4b). Returns `(rendered, sigs)` so the
/// caller can extend `EngineContext.crp_sigs` for legend aggregation. Only
/// called for `Compact`/`Tdd` — `Off` keeps delegating to the core handler for
/// byte-identity. An optional `kind` filter matches `Signature.kind` exactly.
pub fn render_file_signatures(
    content: &str,
    ext: &str,
    mode: CrpMode,
    kind: Option<&str>,
) -> (String, Vec<Signature>) {
    let mut sigs = extract_signatures(content, ext);
    if let Some(k) = kind.filter(|k| *k != "all") {
        sigs.retain(|s| s.kind == k);
    }
    let rendered = sigs
        .iter()
        .map(|s| match mode {
            CrpMode::Tdd => s.to_tdd_located(),
            _ => s.to_compact_located(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    (rendered, sigs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::protocol::CrpMode;

    #[test]
    fn output_rules_empty_for_off_nonempty_for_dense() {
        assert!(crp_output_rules(CrpMode::Off).is_empty());
        assert!(!crp_output_rules(CrpMode::Compact).is_empty());
        assert_eq!(
            crp_output_rules(CrpMode::Compact),
            crp_output_rules(CrpMode::Tdd),
            "rules derive from the single tdd-schema source"
        );
    }

    #[test]
    fn guidance_block_is_stable_and_headed() {
        assert_eq!(crp_guidance_block(CrpMode::Off), "");
        let tdd = crp_guidance_block(CrpMode::Tdd);
        assert!(
            tdd.starts_with("<!-- crp:tdd -->\n"),
            "stable header: {tdd}"
        );
        assert!(tdd.contains("- "), "rules rendered as bullets: {tdd}");
        // #498: pure function of mode → byte-identical across calls.
        assert_eq!(tdd, crp_guidance_block(CrpMode::Tdd));
        assert!(crp_guidance_block(CrpMode::Compact).starts_with("<!-- crp:compact -->\n"));
    }

    #[test]
    fn human_legend_expands_glyphs_to_words() {
        use crate::core::signatures::Signature;
        let mut s = Signature::no_span();
        s.kind = "fn";
        let refs: Vec<&Signature> = vec![&s];
        let legend = human_legend(&refs);
        assert!(legend.contains("Funktion"), "fn → Funktion: {legend}");
        assert!(
            !legend.contains('λ'),
            "no dense glyphs in human legend: {legend}"
        );

        // is_exported bucket
        let mut se = Signature::no_span();
        se.kind = "fn";
        se.is_exported = true;
        let refs_e: Vec<&Signature> = vec![&se];
        let legend_e = human_legend(&refs_e);
        assert!(
            legend_e.contains("öffentlich"),
            "is_exported → öffentlich: {legend_e}"
        );
        assert!(
            !legend_e.contains('+'),
            "no dense glyph + in human legend: {legend_e}"
        );

        // is_async bucket
        let mut sa = Signature::no_span();
        sa.kind = "fn";
        sa.is_async = true;
        let refs_a: Vec<&Signature> = vec![&sa];
        let legend_a = human_legend(&refs_a);
        assert!(
            legend_a.contains("asynchron"),
            "is_async → asynchron: {legend_a}"
        );
        assert!(
            !legend_a.contains('~'),
            "no dense glyph ~ in human legend: {legend_a}"
        );
    }

    #[test]
    fn human_legend_empty_for_no_sigs() {
        assert_eq!(human_legend(&[]), "");
    }

    #[test]
    fn i1_every_tdd_glyph_has_a_legend_entry() {
        use crate::core::signatures::{Signature, tdd_legend};
        // One signature per kind that to_tdd can emit, plus pub + async markers.
        let kinds = [
            "fn",
            "method",
            "class",
            "struct",
            "interface",
            "trait",
            "type",
            "enum",
            "const",
            "let",
            "var",
        ];
        let sigs: Vec<Signature> = kinds
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let mut s = Signature::no_span();
                s.kind = *k;
                s.name = format!("s{i}");
                s.is_exported = true; // '+' marker
                s.is_async = k == &"fn"; // '~' marker
                s
            })
            .collect();
        let refs: Vec<&Signature> = sigs.iter().collect();
        let legend = tdd_legend(&refs);
        // Every distinct glyph produced by to_tdd must be explained in the legend.
        for s in &sigs {
            let rendered = s.to_tdd();
            for glyph in ['λ', '§', '∂', 'τ', 'ε', 'ν', '+', '~'] {
                if rendered.contains(glyph) {
                    assert!(
                        legend.contains(glyph),
                        "glyph {glyph} from {rendered:?} missing in legend {legend:?}"
                    );
                }
            }
        }
    }
}
