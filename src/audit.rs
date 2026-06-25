//! Executable R/H/E necessity-audit for lmd directives (spec §3.1).

/// Necessity classification for an lmd directive (spec §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveClass {
    /// Thin alias over an existing lean-ctx core API. No new logic.
    Router,
    /// Already handled (better) by the hook layer; engine must not double-track.
    /// Taxonomy-only in v1: no table entry yet; exercised by the backing test.
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
    /// Backing identifier: either a crate-relative `src/` path (Task 3 asserts
    /// it exists) or a well-known external name (`std`, `chrono`, `glob`,
    /// `rushdown`). Empty only for pure-Extension entries with no stable anchor.
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
            backing: "src/bridges/read.rs",
            est_bridge_lines: 20,
            note: "routes outbound to ctx_read via ctx.backend",
        },
        DirectiveAudit {
            directive: "@search",
            class: Router,
            backing: "src/bridges/search.rs",
            est_bridge_lines: 20,
            note: "routes outbound to ctx_search via ctx.backend",
        },
        DirectiveAudit {
            directive: "@list",
            class: Router,
            backing: "src/bridges/list.rs",
            est_bridge_lines: 20,
            note: "routes outbound to ctx_tree via ctx.backend",
        },
        DirectiveAudit {
            directive: "@query",
            class: Router,
            backing: "src/bridges/query.rs",
            est_bridge_lines: 30,
            note: "routes outbound to ctx_shell; same allowlist/redaction (security gate §7)",
        },
        DirectiveAudit {
            directive: "@graph",
            class: Router,
            backing: "src/bridges/graph.rs",
            est_bridge_lines: 80,
            note: "7 ops routed outbound to ctx_graph/ctx_callgraph via ctx.backend",
        },
        DirectiveAudit {
            directive: "@remember",
            class: Router,
            backing: "src/bridges/remember.rs",
            est_bridge_lines: 15,
            note: "routes outbound to ctx_knowledge remember; profile=skill only (§7)",
        },
        DirectiveAudit {
            directive: "@recall",
            class: Router,
            backing: "src/bridges/recall.rs",
            est_bridge_lines: 15,
            note: "routes outbound to ctx_knowledge recall_for_output, no_track",
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
            backing: "src/bridges/count.rs",
            est_bridge_lines: 10,
            note: "in-process std::fs glob walk (no backend, no glob crate)",
        },
        DirectiveAudit {
            directive: "@phase",
            class: RouterHook,
            backing: "src/phases.rs",
            est_bridge_lines: 25,
            note: "phase executor; sinks route outbound to ctx_session decision/finding",
        },
        DirectiveAudit {
            directive: "@on complete",
            class: RouterHook,
            backing: "src/phases.rs",
            est_bridge_lines: 15,
            note: "@on complete sinks route outbound to ctx_session/ctx_knowledge/ctx_agent",
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
            backing: "src/crp_schema.rs",
            est_bridge_lines: 35,
            note: "vendored crp_schema (R) + render hook (E); modes tdd/compact/off",
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

    #[test]
    fn audit_backing_files_exist() {
        // CARGO_MANIFEST_DIR is the `rust/` crate root at compile time.
        let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for entry in directive_audit() {
            if entry.backing.starts_with("src/") {
                let path = crate_root.join(entry.backing);
                assert!(
                    path.exists(),
                    "backing file `{}` for directive `{}` does not exist — \
                     the verified code anchor moved; update the audit",
                    entry.backing,
                    entry.directive
                );
            }
        }
    }
}
