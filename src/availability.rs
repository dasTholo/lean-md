//! Tool-availability audit (Spec §5.4 "Tool availability audit" + §8 #12).
//! COVERAGE maps each brainstorming workflow step → lmd directive → lean-ctx
//! backing; the gate asserts every covered directive is registered. GAP_LIST
//! names tools intentionally NOT in the brainstorming path (transparency, not a
//! silent hole). Byte-stable (#498).

/// (skill, workflow step, lmd directive name as in default_registry, lean-ctx backing).
pub const COVERAGE: &[(&str, &str, &str, &str)] = &[
    ("lmd-brainstorm", "explore", "read", "ctx_read"),
    ("lmd-brainstorm", "explore", "list", "ctx_tree"),
    ("lmd-brainstorm", "explore", "search", "ctx_search"),
    ("lmd-brainstorm", "explore", "find", "ctx_semantic_search"),
    ("lmd-brainstorm", "approaches", "graph", "graph_index"),
    ("lmd-brainstorm", "approaches", "impact", "ctx_impact"),
    ("lmd-brainstorm", "write-spec", "edit", "ctx_edit"),
    ("lmd-brainstorm", "write-spec", "remember", "ctx_knowledge"),
    ("lmd-brainstorm", "self-review", "review", "ctx_review"),
    ("lmd-brainstorm", "handoff", "dispatch", "fragment-compose"),
    ("lmd-brainstorm", "handoff", "handoff", "ctx_handoff"),
    // TDD is prose-discipline + directive-arm: the RED phase reads the test/impl.
    // Test execution (`ctx_shell "cargo nextest run"`) is NOT a registered
    // directive — see GAP_LIST note below.
    ("lmd-test-driven-development", "red", "read", "ctx_read"),
    // Companion (Spec #2): the testing-anti-patterns reference pulls the
    // discipline block via `@include test-first-core` (the include directive).
    (
        "lmd-test-driven-development",
        "testing-anti-patterns",
        "include",
        "fragment-compose",
    ),
    // writing-skills is prose-discipline: the RED baseline reads the skill/test.
    ("lmd-writing-skills", "red", "read", "ctx_read"),
    // Discipline companion pulls the trip-wire via `@include skill-authoring-core`.
    (
        "lmd-writing-skills",
        "testing/methodology",
        "include",
        "fragment-compose",
    ),
    // green phase dispatches the tester subagent (brief = testing/methodology).
    (
        "lmd-writing-skills",
        "green",
        "dispatch",
        "fragment-compose",
    ),
];

/// Tools deliberately outside the brainstorming directive surface. Note: TDD's
/// test execution (`ctx_shell "cargo nextest run"`) is also intentionally NOT a
/// registered directive — it is raw shell, not a code-intel directive (TDD is
/// prose-discipline). Recorded here for transparency, not added to the list.
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
        for (skill, step, directive, backing) in COVERAGE {
            assert!(
                reg.get(directive).is_some(),
                "directive '{directive}' (skill={skill}, step={step}, backing={backing}) not in default_registry()"
            );
        }
    }

    #[test]
    fn coverage_carries_skill_dimension() {
        // Both skills must appear; every TDD row's directive must be registered.
        let skills: std::collections::HashSet<&str> =
            COVERAGE.iter().map(|(skill, _, _, _)| *skill).collect();
        assert!(skills.contains("lmd-brainstorm"));
        assert!(skills.contains("lmd-test-driven-development"));
        assert!(skills.contains("lmd-writing-skills"));
    }

    #[test]
    fn gap_list_is_byte_stable() {
        // Deterministic snapshot of the deliberately-excluded tools (#498).
        assert_eq!(
            gap_list_rendered(),
            "ctx_benchmark\nctx_package\nctx_provider\n"
        );
    }

    #[test]
    fn coverage_carries_companion_row() {
        let has_companion = COVERAGE.iter().any(|(skill, step, directive, _)| {
            *skill == "lmd-test-driven-development"
                && *step == "testing-anti-patterns"
                && *directive == "include"
        });
        assert!(
            has_companion,
            "COVERAGE must record the companion @include row"
        );
    }
}
