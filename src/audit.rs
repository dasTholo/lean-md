//! Executable R/H/E necessity-audit for lmd directives (spec §3.1).

/// Necessity classification for an lmd directive (spec §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveClass {
    /// Thin alias over an existing lean-ctx core API. No new logic.
    Router,
    /// Already handled (better) by the hook layer; engine must not double-track.
    Hook,
    /// A genuine rushdown engine construct with no lean-ctx equivalent.
    Extension,
    /// Router behavior plus a hook double-tracking check.
    RouterHook,
    /// Router data plus a render-side extension hook.
    RouterExtension,
}

/// One row of the executable necessity-audit.
#[derive(Debug, Clone)]
pub struct DirectiveAudit {
    /// Directive token as written in an `.lmd.md` source.
    pub directive: &'static str,
    /// R / H / E classification.
    pub class: DirectiveClass,
    /// For Router/Hook directives: crate-relative source file of the backing
    /// API (e.g. `src/core/structured_read.rs`). `std` / `rushdown` / `chrono`
    /// for non-source backings. Empty only for pure-Extension entries with no
    /// stable anchor yet.
    pub backing: &'static str,
    /// Rough bridge-size estimate in lines (spec §3.1 "Bridge-Zeilenschätzung").
    pub est_bridge_lines: u32,
    /// Free-form note: H-checks, deferred ops, fallbacks.
    pub note: &'static str,
}

/// The full v1 necessity-audit table (spec §3.1). This is the executable
/// artifact: `audit_backing_files_exist` (Task 3) asserts every `src/...`
/// backing actually resolves on disk, turning the audit into a CI guard
/// against anchor drift.
#[must_use]
pub fn directive_audit() -> Vec<DirectiveAudit> {
    use DirectiveClass::{Extension, Router, RouterExtension, RouterHook};
    vec![
        DirectiveAudit {
            directive: "@read",
            class: Router,
            backing: "src/core/structured_read.rs",
            est_bridge_lines: 20,
            note: "routes to core::structured_read / ctx_read",
        },
        DirectiveAudit {
            directive: "@search",
            class: Router,
            backing: "src/tools/ctx_search.rs",
            est_bridge_lines: 20,
            note: "routes to ctx_search",
        },
        DirectiveAudit {
            directive: "@list",
            class: Router,
            backing: "src/tools/ctx_tree.rs",
            est_bridge_lines: 20,
            note: "routes to ctx_tree",
        },
        DirectiveAudit {
            directive: "@query",
            class: Router,
            backing: "src/shell/exec.rs",
            est_bridge_lines: 30,
            note: "shell/exec + compress; same allowlist/redaction as ctx_shell (security gate §7)",
        },
        DirectiveAudit {
            directive: "@graph",
            class: Router,
            backing: "src/core/graph_index.rs",
            est_bridge_lines: 80,
            note: "7 ops via graph_index/call_graph/graph_context; recent-neighbors gated on G-1",
        },
        DirectiveAudit {
            directive: "@remember",
            class: Router,
            backing: "src/core/knowledge/core.rs",
            est_bridge_lines: 15,
            note: "ctx_knowledge remember; profile=skill only (§7)",
        },
        DirectiveAudit {
            directive: "@recall",
            class: Router,
            backing: "src/core/knowledge/query.rs",
            est_bridge_lines: 15,
            note: "ctx_knowledge recall_for_output, no_track",
        },
        DirectiveAudit {
            directive: "@env",
            class: Router,
            backing: "std",
            est_bridge_lines: 8,
            note: "std::env",
        },
        DirectiveAudit {
            directive: "@date",
            class: Router,
            backing: "chrono",
            est_bridge_lines: 8,
            note: "chrono (already a dep)",
        },
        DirectiveAudit {
            directive: "@count",
            class: Router,
            backing: "glob",
            est_bridge_lines: 10,
            note: "glob (already a dep)",
        },
        DirectiveAudit {
            directive: "@phase",
            class: RouterHook,
            backing: "src/core/session/state.rs",
            est_bridge_lines: 25,
            note: "session add_decision/add_finding; H-check: does a hook already track this?",
        },
        DirectiveAudit {
            directive: "@on complete",
            class: RouterHook,
            backing: "src/core/session/state.rs",
            est_bridge_lines: 15,
            note: "session add_finding; same H-check as @phase",
        },
        DirectiveAudit {
            directive: "@lean-md",
            class: Extension,
            backing: "",
            est_bridge_lines: 40,
            note: "header config parser",
        },
        DirectiveAudit {
            directive: "@include",
            class: Extension,
            backing: "",
            est_bridge_lines: 60,
            note: "file inline (content visible) + jail (§7)",
        },
        DirectiveAudit {
            directive: "@import",
            class: Extension,
            backing: "",
            est_bridge_lines: 40,
            note: "definitions-only scope + jail",
        },
        DirectiveAudit {
            directive: "@define",
            class: Extension,
            backing: "",
            est_bridge_lines: 70,
            note: "macro engine; no lean-ctx equivalent",
        },
        DirectiveAudit {
            directive: "@call",
            class: Extension,
            backing: "",
            est_bridge_lines: 50,
            note: "macro invocation with param substitution",
        },
        DirectiveAudit {
            directive: "@if",
            class: Extension,
            backing: "",
            est_bridge_lines: 60,
            note: "container transformer + evalexpr (Phase 3 dep)",
        },
        DirectiveAudit {
            directive: "@consumer",
            class: Extension,
            backing: "",
            est_bridge_lines: 30,
            note: "ai/human audience transformer only (§10)",
        },
        DirectiveAudit {
            directive: "{{ expr }}",
            class: Extension,
            backing: "",
            est_bridge_lines: 40,
            note: "inline eval / AstTransformer",
        },
        DirectiveAudit {
            directive: "@render",
            class: Extension,
            backing: "",
            est_bridge_lines: 40,
            note: "postfix pipe AstTransformer (| @render type=table)",
        },
        DirectiveAudit {
            directive: "tdd-output",
            class: RouterExtension,
            backing: "src/core/tdd_schema.rs",
            est_bridge_lines: 35,
            note: "tdd_schema (R) + render hook (E); modes tdd/compact/off",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact set of directives the v0.7 spec (§3.1) enumerates. This
    /// sentinel list is the contract: if the audit drifts from the spec,
    /// this test fails.
    const SPEC_DIRECTIVES: &[&str] = &[
        "@read",
        "@search",
        "@list",
        "@query",
        "@graph",
        "@remember",
        "@recall",
        "@env",
        "@date",
        "@count",
        "@phase",
        "@on complete",
        "@lean-md",
        "@include",
        "@import",
        "@define",
        "@call",
        "@if",
        "@consumer",
        "{{ expr }}",
        "@render",
        "tdd-output",
    ];

    #[test]
    fn audit_covers_every_spec_directive() {
        let audit = directive_audit();
        let names: Vec<&str> = audit.iter().map(|d| d.directive).collect();
        for expected in SPEC_DIRECTIVES {
            assert!(
                names.contains(expected),
                "audit is missing spec directive `{expected}`"
            );
        }
        assert_eq!(
            audit.len(),
            SPEC_DIRECTIVES.len(),
            "audit has entries not present in the spec sentinel list"
        );
    }

    #[test]
    fn router_and_hook_directives_have_a_backing() {
        for entry in directive_audit() {
            if matches!(
                entry.class,
                DirectiveClass::Router | DirectiveClass::Hook | DirectiveClass::RouterHook
            ) {
                assert!(
                    !entry.backing.is_empty(),
                    "directive `{}` is Router/Hook but has no backing",
                    entry.directive
                );
            }
        }
    }
}
