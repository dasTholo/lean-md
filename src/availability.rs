//! Tool-availability audit (Spec §5.4 "Tool-Verfügbarkeits-Audit" + §8 #12).
//! COVERAGE maps each brainstorming workflow step → lmd directive → lean-ctx
//! backing; the gate asserts every covered directive is registered. GAP_LIST
//! names tools intentionally NOT in the brainstorming path (transparency, not a
//! silent hole). Byte-stable (#498).

/// (workflow step, lmd directive name as in default_registry, lean-ctx backing).
pub const COVERAGE: &[(&str, &str, &str)] = &[
    ("explore", "read", "ctx_read"),
    ("explore", "list", "ctx_tree"),
    ("explore", "search", "ctx_search"),
    ("explore", "find", "ctx_semantic_search"),
    ("approaches", "graph", "graph_index"),
    ("approaches", "impact", "ctx_impact"),
    ("write-spec", "edit", "ctx_edit"),
    ("write-spec", "remember", "ctx_knowledge"),
    ("self-review", "review", "ctx_review"),
    ("handoff", "dispatch", "fragment-compose"),
    ("handoff", "handoff", "ctx_handoff"),
];

/// Tools deliberately outside the brainstorming directive surface.
pub const GAP_LIST: &[&str] = &["ctx_benchmark", "ctx_package", "ctx_provider"];

/// Deterministic, sorted rendering of the gap list (one tool per line).
pub fn gap_list_rendered() -> String {
    let mut items: Vec<&str> = GAP_LIST.to_vec();
    items.sort_unstable();
    let mut out = String::new();
    for it in items {
        out.push_str(it);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_covered_directive_is_registered() {
        // §8 #12: every brainstorming-path directive must exist in the registry.
        let reg = crate::bridges::default_registry();
        for (step, directive, backing) in COVERAGE {
            assert!(
                reg.get(directive).is_some(),
                "directive '{directive}' (step={step}, backing={backing}) not in default_registry()"
            );
        }
    }

    #[test]
    fn gap_list_is_byte_stable() {
        // Deterministic snapshot of the deliberately-excluded tools (#498).
        assert_eq!(
            gap_list_rendered(),
            "ctx_benchmark\nctx_package\nctx_provider\n"
        );
    }
}
