//! lmd skill registry + phase-isolated render-on-invoke (Spec §5.4).
//! Bodies and companions resolve through the `skill_source` cascade
//! (overlay → pack store → debug fallback, #727), byte-stable per #498;
//! `ctx_md_render(skill, phase)` renders one isolated phase against them.

use std::path::PathBuf;
use std::rc::Rc;

use crate::crp_proto::CrpMode;
use crate::engine::{EngineContext, render_body};
use crate::header::{Consumer, parse_header};

use std::path::Path;

/// Registry of lmd skill names. The body path is derived, not tabled:
/// `<name>/body.lmd.md`, relative to the skill-content root.
pub const SKILLS: &[&str] = &[
    "lmd-brainstorm",
    "lmd-test-driven-development",
    "lmd-writing-skills",
    "lmd-writing-plans",
    "lmd-subagent-driven-development",
    "lmd-executing-plans",
    "lmd-finishing-a-development-branch",
    "lmd-dispatching-parallel-agents",
];

/// Registry of `(skill, companion)` pairs. The path is derived:
/// `<skill>/companions/<companion>.lmd.md` — a companion name may carry a
/// subdirectory (`testing/methodology`), which keeps the rule uniform.
pub const COMPANIONS: &[(&str, &str)] = &[
    ("lmd-test-driven-development", "testing-anti-patterns"),
    ("lmd-writing-skills", "skill-anatomy"),
    ("lmd-writing-skills", "skill-discovery-optimization"),
    ("lmd-writing-skills", "bulletproofing"),
    ("lmd-writing-skills", "testing/methodology"),
    ("lmd-writing-skills", "testing/skill-types"),
    ("lmd-writing-skills", "testing/creation-checklist"),
    ("lmd-writing-skills", "claude-md-testing-example"),
    ("lmd-writing-skills", "flowchart-conventions"),
    ("lmd-writing-skills", "anthropic-best-practices"),
    ("lmd-writing-skills", "persuasion-principles"),
    ("lmd-brainstorm", "spec-reviewer"),
    ("lmd-brainstorm", "visual-companion"),
    ("lmd-writing-plans", "plan-reviewer"),
    ("lmd-subagent-driven-development", "implementer"),
    ("lmd-subagent-driven-development", "task-reviewer"),
    ("lmd-subagent-driven-development", "code-reviewer"),
];

/// Body source of a known lmd skill, resolved through the content cascade.
pub fn skill_source(name: &str, jail_root: &Path) -> Result<String, SkillRenderError> {
    if !SKILLS.contains(&name) {
        return Err(SkillRenderError::UnknownSkill(name.to_string()));
    }
    crate::skill_source::read_skill_file(&format!("{name}/body.lmd.md"), jail_root)
        .map_err(SkillRenderError::Source)
}

/// All skill bodies (for cross-skill `@var` aggregation in `vars --init`).
pub fn all_skill_sources(jail_root: &Path) -> Result<Vec<String>, SkillRenderError> {
    SKILLS.iter().map(|n| skill_source(n, jail_root)).collect()
}

/// Source of a known `(skill, companion)` pair, resolved through the cascade.
pub fn companion_source(
    skill: &str,
    companion: &str,
    jail_root: &Path,
) -> Result<String, SkillRenderError> {
    if !COMPANIONS
        .iter()
        .any(|(s, c)| *s == skill && *c == companion)
    {
        return Err(SkillRenderError::CompanionNotFound(format!(
            "{skill}/{companion}"
        )));
    }
    let rel = format!("{skill}/companions/{companion}.lmd.md");
    crate::skill_source::read_skill_file(&rel, jail_root).map_err(SkillRenderError::Source)
}

#[derive(Debug)]
pub enum SkillRenderError {
    UnknownSkill(String),
    PhaseNotFound(String),
    CompanionNotFound(String),
    Source(crate::skill_source::SourceError),
}

impl std::fmt::Display for SkillRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillRenderError::UnknownSkill(s) => write!(f, "UNKNOWN_SKILL '{s}'"),
            SkillRenderError::PhaseNotFound(p) => write!(f, "PHASE_NOT_FOUND '{p}'"),
            SkillRenderError::CompanionNotFound(c) => write!(f, "COMPANION_NOT_FOUND '{c}'"),
            SkillRenderError::Source(e) => write!(f, "{e}"),
        }
    }
}

/// Render an embedded skill body, optionally isolated to a single named phase.
/// `phase=None` renders the full body; `Some(p)` renders ONLY phase `p`
/// (populated via the `capture_phase_bodies` pre-pass — no cross-phase leak).
pub fn render_skill(
    name: &str,
    phase: Option<&str>,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    jail_root: PathBuf,
) -> Result<String, SkillRenderError> {
    let owned = skill_source(name, &jail_root)?;
    let (mut header, body) = parse_header(&owned);
    if let Some(c) = consumer {
        header.consumer = c;
    }
    if let Some(m) = crp {
        header.crp = m;
    }
    let ctx = Rc::new(EngineContext::new(header, jail_root));

    // `@var` pre-pass (Spec): seed the override layer from `vars.toml`, then fill
    // `@var …default=` defaults from the FULL body (default-if-absent). Runs on the
    // full body so isolated phases see vars declared at body-top (outside phases).
    ctx.vars_seed(crate::skill_vars::load_vars(&ctx.jail_root));
    for decl in crate::skill_vars::scan_var_decls(body) {
        ctx.var_set_default(&decl.name, &decl.default);
    }

    match phase {
        None => Ok(render_body(&ctx, body)),
        Some(p) => {
            // Populate phase_bodies from the full body, then render the isolated one.
            crate::phases::capture_phase_bodies(&ctx, body);
            let isolated = ctx
                .phase_body(p)
                .ok_or_else(|| SkillRenderError::PhaseNotFound(p.to_string()))?;
            Ok(render_body(&ctx, &isolated))
        }
    }
}

/// Render an arbitrary `.lmd.md` `source` with the same header-parse, var-prepass
/// and phase-isolation as `render_skill`, but source-agnostic — used to render one
/// task-phase of a generated `.lmd.md` implementation plan. `jail_root` MUST be the
/// project root (cwd), not the source file's parent, so `.lean-ctx/lean-md/vars.toml`
/// and `@import` targets resolve (Spec §4 jail decision).
pub fn render_source_with_phase(
    source: &str,
    phase: Option<&str>,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    jail_root: PathBuf,
) -> Result<String, SkillRenderError> {
    let (mut header, body) = parse_header(source);
    if let Some(c) = consumer {
        header.consumer = c;
    }
    if let Some(m) = crp {
        header.crp = m;
    }
    let ctx = Rc::new(EngineContext::new(header, jail_root));

    // var-prepass — identical to render_skill.
    ctx.vars_seed(crate::skill_vars::load_vars(&ctx.jail_root));
    for decl in crate::skill_vars::scan_var_decls(body) {
        ctx.var_set_default(&decl.name, &decl.default);
    }

    match phase {
        None => Ok(render_body(&ctx, body)),
        Some(p) => {
            // Warm the macro registry from the WHOLE document first: a plan's
            // meta-head @define/@import live outside the @phase blocks, so a @call
            // inside the isolated task-phase would otherwise see an empty registry.
            // Output is discarded — only ctx.macros is populated.
            let _ = crate::macros::extract_definitions(&ctx, body);
            crate::phases::capture_phase_bodies(&ctx, body);
            let isolated = ctx
                .phase_body(p)
                .ok_or_else(|| SkillRenderError::PhaseNotFound(p.to_string()))?;
            Ok(render_body(&ctx, &isolated))
        }
    }
}

/// Render a full body source flat (no phase capture): header overrides + the
/// `@var` pre-pass (default-if-absent), then a single `render_body` pass.
/// Used only by `render_companion` (kept separate from `render_skill` on purpose —
/// `render_skill` must keep its own ctx for `capture_phase_bodies`).
fn render_full_source(
    src: &str,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    jail_root: PathBuf,
) -> String {
    let (mut header, body) = parse_header(src);
    if let Some(c) = consumer {
        header.consumer = c;
    }
    if let Some(m) = crp {
        header.crp = m;
    }
    let ctx = Rc::new(EngineContext::new(header, jail_root));
    ctx.vars_seed(crate::skill_vars::load_vars(&ctx.jail_root));
    for decl in crate::skill_vars::scan_var_decls(body) {
        ctx.var_set_default(&decl.name, &decl.default);
    }
    render_body(&ctx, body)
}

/// Render a skill's on-demand companion as one flat block (no phase sequence).
/// Out-of-band like the body; embedded-only (no overlay layer — YAGNI).
pub fn render_companion(
    skill: &str,
    companion: &str,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    jail_root: PathBuf,
) -> Result<String, SkillRenderError> {
    let src = companion_source(skill, companion, &jail_root)?;
    Ok(render_full_source(&src, consumer, crp, jail_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn unknown_skill_errors() {
        let err = render_skill("nope", None, None, None, PathBuf::from(".")).unwrap_err();
        assert!(matches!(err, SkillRenderError::UnknownSkill(_)));
    }

    #[test]
    fn no_body_or_fragment_claims_a_warm_subagent_cache() {
        // A subagent's first ctx_read is never warm: cross-conversation stubs are
        // withheld (lean-ctx #1040). ctx_multi_read buys latency, not tokens.
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let reg = crate::fragments::FragmentRegistry::with_builtins();
        let dispatch = reg.resolve("dispatch-contract", &jail).unwrap();
        let parallel = reg.resolve("parallel-dispatch", &jail).unwrap();

        // Corpus = every skill body plus the two embedded fragment seeds.
        let mut corpus: Vec<(String, String)> = SKILLS
            .iter()
            .map(|name| ((*name).to_string(), skill_source(name, &jail).unwrap()))
            .collect();
        corpus.push(("dispatch-contract".to_string(), dispatch.clone()));
        corpus.push(("parallel-dispatch".to_string(), parallel.clone()));

        for (origin, text) in &corpus {
            let lower = text.to_lowercase();
            for claim in [
                "cache is already shared",
                "cache is shared",
                "shared cache",
                "shared mcp cache",
                "first `ctx_read` hits",
            ] {
                assert!(
                    !lower.contains(claim),
                    "{origin} still claims a warm subagent cache: '{claim}'"
                );
            }
        }

        // The replacement must name the real reason, not just drop the old one.
        assert!(
            parallel.contains("#1040") && parallel.to_lowercase().contains("latency"),
            "parallel-dispatch must carry the #1040 latency-not-tokens rationale"
        );

        // Guard against over-reach: lmd-writing-plans' "warm cache" is the
        // just-in-time resolution of path:line anchors, NOT a subagent's first read.
        let wp = skill_source("lmd-writing-plans", &jail).unwrap();
        assert!(
            wp.contains("warm cache"),
            "lmd-writing-plans' anchor-resolution wording must stay untouched"
        );
    }

    #[test]
    fn brainstorm_all_phases_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in [
            "pre-context",
            "explore",
            "questions",
            "approaches",
            "present-design",
            "write-spec",
            "self-review",
            "handoff",
        ] {
            let out = render_skill("lmd-brainstorm", Some(p), None, None, jail.clone())
                .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(!out.trim().is_empty(), "phase {p} must render non-empty");
        }
        assert!(
            skill_source("lmd-brainstorm", &jail).is_ok(),
            "lmd-brainstorm must be in the SKILLS registry"
        );
    }

    #[test]
    fn brainstorm_phase_isolation_no_cross_phase_leak() {
        let jail = std::path::PathBuf::from(".");
        // (phase, unique marker that must appear, foreign marker that must NOT appear)
        let cases = [
            ("explore", "Understanding the idea", "Spec Self-Review"),
            (
                "approaches",
                "Propose 2-3 approaches",
                "Implementation handoff",
            ),
            ("self-review", "Spec Self-Review", "Understanding the idea"),
            (
                "handoff",
                "Implementation handoff",
                "Propose 2-3 approaches",
            ),
        ];
        for (phase, own, foreign) in cases {
            let out =
                render_skill("lmd-brainstorm", Some(phase), None, None, jail.clone()).unwrap();
            assert!(out.contains(own), "phase {phase} missing own marker: {out}");
            assert!(
                !out.contains(foreign),
                "cross-phase leak in {phase}: found foreign marker {foreign}"
            );
        }
    }

    #[test]
    fn brainstorm_gate_trip_wire() {
        let jail = std::path::PathBuf::from(".");
        const GATE: &str = "regardless of perceived simplicity";
        // Discipline phases carry the HARD-GATE via @include brainstorm-gate.
        for p in [
            "pre-context",
            "explore",
            "questions",
            "approaches",
            "present-design",
        ] {
            let out = render_skill("lmd-brainstorm", Some(p), None, None, jail.clone()).unwrap();
            assert!(
                out.contains(GATE),
                "discipline phase {p} must carry the gate"
            );
        }
        // Post-approval phases are NOT gate phases.
        for p in ["write-spec", "self-review", "handoff"] {
            let out = render_skill("lmd-brainstorm", Some(p), None, None, jail.clone()).unwrap();
            assert!(
                !out.contains(GATE),
                "non-gate phase {p} must not carry the gate"
            );
        }
    }

    #[test]
    fn unknown_phase_errors() {
        let err = render_skill(
            "lmd-brainstorm",
            Some("does-not-exist"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap_err();
        assert!(matches!(err, SkillRenderError::PhaseNotFound(_)));
    }

    #[test]
    fn registry_resolves_both_skills() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(skill_source("lmd-brainstorm", &jail).is_ok());
        assert!(skill_source("lmd-test-driven-development", &jail).is_ok());
        assert!(skill_source("nope", &jail).is_err());
    }

    #[test]
    fn skill_source_resolves_every_registered_skill() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for name in SKILLS {
            assert!(
                skill_source(name, &jail).is_ok(),
                "registered skill {name} must resolve through the cascade"
            );
        }
    }

    #[test]
    fn companion_source_resolves_every_registered_companion() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (skill, companion) in COMPANIONS {
            assert!(
                companion_source(skill, companion, &jail).is_ok(),
                "registered companion {skill}/{companion} must resolve"
            );
        }
    }

    #[test]
    fn unregistered_skill_is_unknown_never_a_disk_read() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let err = skill_source("lmd-not-a-skill", &jail).unwrap_err();
        assert!(
            matches!(err, SkillRenderError::UnknownSkill(_)),
            "unregistered name must short-circuit to UnknownSkill, not touch disk"
        );
    }

    #[test]
    fn tdd_phases_render_isolated_no_cross_leak() {
        for (phase, marker, foreign) in [
            ("red", "Verify RED", "Common Rationalizations"),
            ("green", "Verify GREEN", "Verify RED"),
            ("refactor", "only under green", "Verify GREEN"),
            ("rationalizations", "Common Rationalizations", "Verify RED"),
        ] {
            let out = render_skill(
                "lmd-test-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(
                out.contains(marker),
                "phase {phase} missing its marker: {out}"
            );
            assert!(
                !out.contains(foreign),
                "phase {phase} leaked foreign content '{foreign}': {out}"
            );
        }
    }

    #[test]
    fn executing_plans_all_phases_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in [
            "orient",
            "preflight",
            "execute",
            "checkpoint",
            "final-gate",
            "finish",
        ] {
            let out = render_skill("lmd-executing-plans", Some(p), None, None, jail.clone())
                .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(!out.trim().is_empty(), "phase {p} must render non-empty");
        }
        assert!(
            skill_source("lmd-executing-plans", &jail).is_ok(),
            "lmd-executing-plans must be in the SKILLS registry"
        );
        // Reference-closure (Global Constraint): the body is a native port — it must not
        // carry the upstream `superpowers` token. This is the body's half of the test gate
        // (SKILL.md's half is asserted in Task 2's install test).
        assert!(
            !skill_source("lmd-executing-plans", &jail)
                .unwrap()
                .to_lowercase()
                .contains("superpowers"),
            "body seed must be reference-closed (no superpowers token)"
        );
    }

    #[test]
    fn executing_plans_orient_carries_hard_rules_baseline() {
        // orient @include hard-rules → the ambient baseline must be present inline,
        // and it must render clean (no unfilled dispatch-contract {{ }} eval errors).
        let out = render_skill(
            "lmd-executing-plans",
            Some("orient"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("Hard Rules (lmd built-in)"),
            "orient must inline the ambient baseline via @include hard-rules: {out}"
        );
        assert!(
            !out.contains("eval err:"),
            "orient must render clean — no unfilled template-var eval errors: {out}"
        );
    }

    #[test]
    fn executing_plans_phase_isolation_no_cross_leak() {
        let jail = std::path::PathBuf::from(".");
        // (phase, own marker, foreign marker that must NOT appear)
        for (phase, own, foreign) in [
            ("execute", "per-task loop", "whole-branch review"),
            ("final-gate", "whole-branch review", "per-task loop"),
            ("finish", "branch completion", "per-task loop"),
        ] {
            let out =
                render_skill("lmd-executing-plans", Some(phase), None, None, jail.clone()).unwrap();
            assert!(out.contains(own), "phase {phase} missing own marker: {out}");
            assert!(
                !out.contains(foreign),
                "cross-phase leak in {phase}: found foreign marker {foreign}"
            );
        }
    }

    #[test]
    fn finishing_all_phases_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in [
            "pre-context",
            "verify-tests",
            "detect-env",
            "present-options",
            "merge-local",
            "create-pr",
            "keep-as-is",
            "discard",
        ] {
            let out = render_skill(
                "lmd-finishing-a-development-branch",
                Some(p),
                None,
                None,
                jail.clone(),
            )
            .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(!out.trim().is_empty(), "phase {p} must render non-empty");
        }
        assert!(
            skill_source("lmd-finishing-a-development-branch", &jail).is_ok(),
            "lmd-finishing-a-development-branch must be in the SKILLS registry"
        );
        assert!(
            !skill_source("lmd-finishing-a-development-branch", &jail)
                .unwrap()
                .to_lowercase()
                .contains("superpowers"),
            "body seed must be reference-closed (no superpowers token)"
        );
    }

    #[test]
    fn finishing_pre_context_carries_hard_rules_baseline() {
        let out = render_skill(
            "lmd-finishing-a-development-branch",
            Some("pre-context"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("Hard Rules (lmd built-in)"),
            "pre-context must inline the ambient baseline via @include hard-rules: {out}"
        );
    }

    #[test]
    fn dispatching_parallel_agents_all_phases_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in ["pre-context", "assess", "dispatch", "integrate"] {
            let out = render_skill(
                "lmd-dispatching-parallel-agents",
                Some(p),
                None,
                None,
                jail.clone(),
            )
            .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(!out.trim().is_empty(), "phase {p} must render non-empty");
        }
        assert!(
            skill_source("lmd-dispatching-parallel-agents", &jail).is_ok(),
            "lmd-dispatching-parallel-agents must be in the SKILLS registry"
        );
        assert!(
            !skill_source("lmd-dispatching-parallel-agents", &jail)
                .unwrap()
                .to_lowercase()
                .contains("superpowers"),
            "body seed must be reference-closed (no superpowers token)"
        );
    }

    #[test]
    fn dispatching_pre_context_carries_hard_rules_baseline() {
        let out = render_skill(
            "lmd-dispatching-parallel-agents",
            Some("pre-context"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("Hard Rules (lmd built-in)"),
            "pre-context must inline the ambient baseline via @include hard-rules: {out}"
        );
    }

    #[test]
    fn dispatching_assess_and_dispatch_carry_parallel_fragment() {
        for p in ["assess", "dispatch"] {
            let out = render_skill(
                "lmd-dispatching-parallel-agents",
                Some(p),
                None,
                None,
                std::path::PathBuf::from("."),
            )
            .unwrap();
            assert!(
                out.contains("one dispatch per independent problem domain"),
                "phase {p} must resolve @include parallel-dispatch (fragment marker): {out}"
            );
        }
    }

    #[test]
    fn dispatching_source_overlay_wins_over_debug_fallback() {
        let root = std::env::temp_dir().join(format!("lmd_casc_disp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md/skills/lmd-dispatching-parallel-agents");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("body.lmd.md"), "OVERLAY_DISPATCH_BODY").unwrap();
        let src = skill_source("lmd-dispatching-parallel-agents", &root).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            src, "OVERLAY_DISPATCH_BODY",
            "overlay must win over the debug fallback"
        );
    }

    #[test]
    fn finishing_merge_local_carries_provenance_cleanup() {
        let out = render_skill(
            "lmd-finishing-a-development-branch",
            Some("merge-local"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("git worktree remove") && out.contains("git worktree prune"),
            "merge-local must carry the provenance cleanup: {out}"
        );
    }

    #[test]
    fn every_tdd_phase_includes_test_first_core() {
        for phase in ["red", "green", "refactor", "rationalizations"] {
            let out = render_skill(
                "lmd-test-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(
                out.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"),
                "phase {phase} must @include test-first-core (Iron Law marker): {out}"
            );
        }
    }

    #[test]
    fn tdd_source_overlay_wins_over_debug_fallback() {
        let root = std::env::temp_dir().join(format!("lmd_casc_tdd_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md/skills/lmd-test-driven-development");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("body.lmd.md"), "OVERLAY_TDD_BODY").unwrap();
        let src = skill_source("lmd-test-driven-development", &root).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            src, "OVERLAY_TDD_BODY",
            "overlay must win over the debug fallback"
        );
    }

    #[test]
    fn body_override_prefers_project_overlay() {
        let root = std::env::temp_dir().join(format!("lmd_body_override_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let overlay_dir = root.join(".lean-ctx/lean-md/skills/lmd-test-driven-development");
        std::fs::create_dir_all(&overlay_dir).unwrap();
        std::fs::write(
            overlay_dir.join("body.lmd.md"),
            "@phase \"red\"\nOVERLAY_RED_MARKER\n@phase-end\n",
        )
        .unwrap();

        let out = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("OVERLAY_RED_MARKER"),
            "overlay body must be rendered when present: {out}"
        );
        assert!(
            !out.contains("Verify RED"),
            "embedded body must NOT be used when overlay exists: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn body_override_falls_back_to_embedded_when_absent() {
        // No overlay under this jail root → embedded body is used.
        let root = std::env::temp_dir().join(format!("lmd_no_overlay_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(out.contains("Verify RED"), "embedded body fallback: {out}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn all_skill_bodies_aggregate_contains_test_cmd_decl() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bodies = all_skill_sources(&jail).unwrap();
        let decls: Vec<_> = bodies
            .iter()
            .flat_map(|b| crate::skill_vars::scan_var_decls(b.as_str()))
            .collect();
        assert!(
            decls.iter().any(|d| d.name == "test_cmd"),
            "aggregating @var across all SKILLS must surface test_cmd"
        );
    }

    /// Write a synthetic overlay body declaring a var at body-top (outside phases)
    /// and using it inside an isolated phase — the phase-isolation crux.
    fn write_var_overlay(root: &std::path::Path) {
        let dir = root.join(".lean-ctx/lean-md/skills/lmd-test-driven-development");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("body.lmd.md"),
            "@var demo_cmd default=\"DEFAULT_VAL\" desc=\"d\"\n\
             @phase \"p1\"\nP1 uses {{ var demo_cmd }} here\n@phase-end\n\
             @phase \"p2\"\nP2_FOREIGN_MARKER\n@phase-end\n",
        )
        .unwrap();
    }

    #[test]
    fn prepass_default_applies_and_phase_isolated() {
        let root = std::env::temp_dir().join(format!("lmd_var_default_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_var_overlay(&root);
        let out = render_skill(
            "lmd-test-driven-development",
            Some("p1"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("DEFAULT_VAL"),
            "@var default must resolve in isolated phase: {out}"
        );
        assert!(
            !out.contains("P2_FOREIGN_MARKER"),
            "no cross-phase leak: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepass_config_overrides_default() {
        let root = std::env::temp_dir().join(format!("lmd_var_override_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_var_overlay(&root);
        std::fs::write(
            root.join(".lean-ctx/lean-md/vars.toml"),
            "demo_cmd = \"OVERRIDE_VAL\"\n",
        )
        .unwrap();
        let out = render_skill(
            "lmd-test-driven-development",
            Some("p1"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("OVERRIDE_VAL"),
            "vars.toml must win over @var default: {out}"
        );
        assert!(
            !out.contains("DEFAULT_VAL"),
            "default must be shadowed by config: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tdd_red_renders_default_test_cmd_without_config() {
        let root = std::env::temp_dir().join(format!("lmd_tdd_default_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("cargo test"),
            "default test_cmd must render: {out}"
        );
        assert!(
            !out.contains("cargo nextest run"),
            "no override without vars.toml: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tdd_red_renders_overridden_test_cmd_with_config() {
        let root = std::env::temp_dir().join(format!("lmd_tdd_override_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx/lean-md")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/lean-md/vars.toml"),
            "test_cmd = \"cargo nextest run\"\n",
        )
        .unwrap();
        let out = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("cargo nextest run"),
            "vars.toml must override: {out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn companion_registry_resolves_testing_anti_patterns() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(
            companion_source(
                "lmd-test-driven-development",
                "testing-anti-patterns",
                &jail
            )
            .is_ok()
        );
        assert!(companion_source("lmd-test-driven-development", "nope", &jail).is_err());
        assert!(companion_source("nope", "testing-anti-patterns", &jail).is_err());
    }

    #[test]
    fn companion_renders_all_anti_pattern_markers() {
        let out = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        for marker in [
            "Anti-Pattern 1",
            "Anti-Pattern 2",
            "Anti-Pattern 3",
            "Anti-Pattern 4",
            "Anti-Pattern 5",
            "Quick Reference",
            "Red Flags",
        ] {
            assert!(out.contains(marker), "companion missing '{marker}': {out}");
        }
    }

    #[test]
    fn companion_includes_test_first_core_iron_law() {
        let out = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"),
            "companion must @include test-first-core (Iron Law marker): {out}"
        );
    }

    #[test]
    fn unknown_companion_errors() {
        let err = render_companion(
            "lmd-test-driven-development",
            "does-not-exist",
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap_err();
        assert!(matches!(err, SkillRenderError::CompanionNotFound(_)));
    }

    #[test]
    fn companion_render_is_deterministic() {
        let jail = PathBuf::from(".");
        let a = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        let b = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            jail,
        )
        .unwrap();
        assert_eq!(a, b, "render_companion must be deterministic (#498)");
    }

    #[test]
    fn companion_source_overlay_wins_over_debug_fallback() {
        let root = std::env::temp_dir().join(format!("lmd_casc_comp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md/skills/lmd-test-driven-development/companions");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("testing-anti-patterns.lmd.md"),
            "OVERLAY_COMPANION",
        )
        .unwrap();
        let src = companion_source(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            &root,
        )
        .unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            src, "OVERLAY_COMPANION",
            "overlay must win over the debug fallback"
        );
    }

    #[test]
    fn no_dangling_companion_refs_in_seeds() {
        use regex::Regex;
        // Skill-scoped directive form, covering both `ctx_md_render(skill="<s>",
        // companion="<c>")` (comma separator) and `@dispatch skill="<s>"
        // companion="<c>"` (whitespace separator). The companion must resolve
        // under that skill — strictly stronger than a bare existence check.
        let call_re = Regex::new(r#"skill="([^"]+)"[,\s]+companion="([^"]+)""#).unwrap();
        // Prose mention form `companion "<c>"` (whitespace, no `=`). The `=`
        // form is fully covered by call_re, so this requires a real separator
        // and never matches the malformed `companion"<c>"`.
        let mention_re = Regex::new(r#"companion\s+"([^"]+)""#).unwrap();

        // Corpus: every embedded skill body PLUS every embedded companion body.
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut corpus: Vec<String> = all_skill_sources(&jail).unwrap();
        for (skill, companion) in COMPANIONS {
            corpus.push(companion_source(skill, companion, &jail).unwrap());
        }

        for body in &corpus {
            // Skill-scoped render calls: companion must resolve under that skill.
            for cap in call_re.captures_iter(body) {
                let skill = &cap[1];
                let companion = &cap[2];
                assert!(
                    companion_source(skill, companion, &jail).is_ok(),
                    "dangling companion ref: skill=\"{skill}\" companion=\"{companion}\""
                );
            }
            // Every companion mention (prose or call) must name a registered companion.
            for cap in mention_re.captures_iter(body) {
                let name = &cap[1];
                assert!(
                    COMPANIONS.iter().any(|(_, c)| *c == name),
                    "dangling companion ref: companion \"{name}\" not in COMPANIONS"
                );
            }
        }
    }

    #[test]
    fn brainstorm_spec_reviewer_resolves() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = companion_source("lmd-brainstorm", "spec-reviewer", &jail)
            .expect("spec-reviewer companion must be registered");
        assert!(
            body.contains("What to Check"),
            "reviewer brief content missing"
        );
        assert!(
            body.contains("Approved | Issues Found"),
            "output format missing"
        );
    }

    #[test]
    fn brainstorm_dispatch_spec_reviewer_composes() {
        // The self-review phase materialises @dispatch companion="spec-reviewer" role=review.
        let out = render_skill(
            "lmd-brainstorm",
            Some("self-review"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(out.contains("role=review"), "review role missing: {out}");
        assert!(
            out.contains("What to Check"),
            "reviewer brief missing: {out}"
        );
        assert!(
            out.contains("to_agent={{ controller_id }}"),
            "controller_id placeholder must survive verbatim: {out}"
        );
        assert!(
            out.contains("ToolSearch(query=\"select:mcp__lean-ctx__ctx_read"),
            "dispatch bootstrap missing: {out}"
        );
    }

    #[test]
    fn brainstorm_visual_companion_resolves() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = companion_source("lmd-brainstorm", "visual-companion", &jail)
            .expect("visual-companion companion must be registered");
        assert!(
            !body.trim().is_empty(),
            "visual-companion must be non-empty"
        );
    }

    #[test]
    fn brainstorm_visual_companion_harness_matrix_verbatim() {
        let out = render_companion(
            "lmd-brainstorm",
            "visual-companion",
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        for platform in [
            "**Claude Code:**",
            "**Codex:**",
            "**Gemini CLI:**",
            "**Copilot CLI:**",
            "**Other environments:**",
        ] {
            assert!(
                out.contains(platform),
                "harness matrix must keep {platform} verbatim (R3)"
            );
        }
        // Reference-closure: no superpowers session path survives.
        assert!(
            !out.contains(".superpowers/"),
            "superpowers session path must be rewritten to .lean-ctx/"
        );
    }

    #[test]
    fn brainstorm_companion_render_is_deterministic() {
        // CLI==MCP (#498): both surfaces call render_companion → byte-identical.
        let jail = std::path::PathBuf::from(".");
        for c in ["spec-reviewer", "visual-companion"] {
            let a = render_companion("lmd-brainstorm", c, None, None, jail.clone()).unwrap();
            let b = render_companion("lmd-brainstorm", c, None, None, jail.clone()).unwrap();
            assert_eq!(a, b, "render_companion({c}) must be deterministic (#498)");
        }
    }

    #[test]
    fn testing_companions_render_for_writing_skills() {
        for companion in ["testing/methodology", "testing/creation-checklist"] {
            let out = render_companion(
                "lmd-writing-skills",
                companion,
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap_or_else(|e| panic!("companion '{companion}' must render: {e:?}"));
            assert!(
                !out.trim().is_empty(),
                "companion '{companion}' rendered empty"
            );
        }
    }

    #[test]
    fn skill_md_stub_carries_orientation() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let stub = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-test-driven-development/SKILL.md"),
        )
        .unwrap();
        // Frontmatter trigger unchanged (SDO/discovery).
        assert!(stub.contains(
            "description: Use when implementing any feature or bugfix, before writing implementation code"
        ));
        // Orientation layer (E6).
        assert!(stub.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"));
        // The render handle is single-sourced in lmd-rendering-skills; the stub only
        // names its phases and companions.
        assert!(stub.contains("lmd-rendering-skills"));
        for name in [
            "**red**",
            "**green**",
            "**refactor**",
            "**rationalizations**",
            "`testing-anti-patterns`",
        ] {
            assert!(stub.contains(name), "stub missing phase/companion '{name}'");
        }
        // Companion trigger (E7, upstream wording) + final rule.
        assert!(stub.contains("When adding mocks or test utilities"));
        assert!(stub.contains("Otherwise → not TDD"));
    }

    #[test]
    fn tdd_render_is_byte_stable_with_config() {
        let root = std::env::temp_dir().join(format!("lmd_tdd_determinism_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx/lean-md")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/lean-md/vars.toml"),
            "test_cmd = \"cargo nextest run\"\n",
        )
        .unwrap();
        let a = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        let b = render_skill(
            "lmd-test-driven-development",
            Some("red"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert_eq!(
            a, b,
            "two renders with same vars.toml must be byte-identical"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rationalizations_points_to_companion_render() {
        let out = render_skill(
            "lmd-test-driven-development",
            Some("rationalizations"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("companion=\"testing-anti-patterns\""),
            "rationalizations must carry the concrete companion render call: {out}"
        );
        assert!(
            !out.contains("ported in Spec #2"),
            "the Spec #2 placeholder must be gone: {out}"
        );
    }

    #[test]
    fn rationalizations_carries_full_fidelity_set() {
        let out = render_skill(
            "lmd-test-driven-development",
            Some("rationalizations"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        // Full 11-row Common Rationalizations — distinctive phrases beyond the old 4 (E12).
        for needle in [
            "Too simple to test",
            "Tests after achieve the same goals",
            "already manually tested",
            "Sunk cost",
            "Keep it as reference",
            "explore first",
            "hard to use",
            "TDD will slow me down",
            "Manual testing is faster",
            "existing code has no tests",
        ] {
            assert!(
                out.contains(needle),
                "rationalizations missing '{needle}': {out}"
            );
        }
        // When-Stuck table restored.
        assert!(
            out.contains("When Stuck"),
            "When-Stuck section missing: {out}"
        );
        assert!(
            out.contains("dependency injection"),
            "When-Stuck must cover the mock-everything → DI cure: {out}"
        );
        // Debugging Integration restored.
        assert!(
            out.contains("Never fix a bug without a test"),
            "Debugging Integration line missing: {out}"
        );
        // Phase isolation still holds (no green/red leak).
        assert!(
            !out.contains("Verify RED"),
            "rationalizations leaked red phase: {out}"
        );
    }

    #[test]
    fn phases_carry_next_pointers() {
        for (phase, needle) in [
            ("red", "next: render phase \"green\""),
            ("green", "next: render phase \"refactor\""),
            ("refactor", "next: render phase \"red\""),
        ] {
            let out = render_skill(
                "lmd-test-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(
                out.contains(needle),
                "phase {phase} missing next-pointer '{needle}': {out}"
            );
        }

        // SDD chain: orient → preflight → dispatch-mode → dispatch | parallel-dispatch
        //            → review → final-review → handoff.
        // Every non-terminal phase carries a next: pointer; handoff is terminal.
        for (phase, needle) in [
            ("orient", "next: render phase \"preflight\""),
            ("preflight", "next: render phase \"dispatch-mode\""),
            ("dispatch-mode", "next: render phase \"dispatch\""),
            ("dispatch", "next: render phase \"review\""),
            ("review", "next: render phase \"final-review\""),
            ("parallel-dispatch", "next: render phase \"final-review\""),
            ("final-review", "next: render phase \"handoff\""),
        ] {
            let out = render_skill(
                "lmd-subagent-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(
                out.contains(needle),
                "SDD phase {phase} missing next-pointer '{needle}': {out}"
            );
        }
        let handoff = render_skill(
            "lmd-subagent-driven-development",
            Some("handoff"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        assert!(
            !handoff.contains("next: render phase"),
            "handoff must be terminal (no next pointer): {handoff}"
        );
    }

    #[test]
    fn writing_skills_is_registered() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(
            skill_source("lmd-writing-skills", &jail).is_ok(),
            "lmd-writing-skills must be in the SKILLS registry"
        );
        let red = render_skill(
            "lmd-writing-skills",
            Some("red"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            red.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "writing-skills phase must carry the Iron Law via @include skill-authoring-core"
        );
    }

    #[test]
    fn writing_skills_all_companions_resolve() {
        let names = [
            "skill-anatomy",
            "skill-discovery-optimization",
            "bulletproofing",
            "testing/methodology",
            "testing/skill-types",
            "testing/creation-checklist",
            "claude-md-testing-example",
            "flowchart-conventions",
            "anthropic-best-practices",
            "persuasion-principles",
        ];
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for n in names {
            let body = companion_source("lmd-writing-skills", n, &jail)
                .unwrap_or_else(|_| panic!("companion {n} not registered"));
            assert!(!body.trim().is_empty(), "companion {n} must be non-empty");
        }
    }

    #[test]
    fn writing_skills_discipline_companions_carry_trip_wire() {
        let jail = std::path::PathBuf::from(".");
        for n in ["testing/methodology", "bulletproofing"] {
            let out = render_companion("lmd-writing-skills", n, None, None, jail.clone()).unwrap();
            assert!(
                out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
                "discipline companion {n} must @include skill-authoring-core"
            );
            assert!(
                !out.contains("writing-skills directory"),
                "discipline companion {n} must not reference the superpowers writing-skills directory (reference-closure)"
            );
        }
    }

    #[test]
    fn writing_skills_phases_are_isolated() {
        let jail = std::path::PathBuf::from(".");
        let red =
            render_skill("lmd-writing-skills", Some("red"), None, None, jail.clone()).unwrap();
        let green = render_skill(
            "lmd-writing-skills",
            Some("green"),
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        // Each phase carries the shared trip-wire...
        assert!(red.contains("NO SKILL WITHOUT A FAILING TEST FIRST"));
        assert!(green.contains("NO SKILL WITHOUT A FAILING TEST FIRST"));
        // ...but NOT the other phase's unique heading (no cross-phase leak).
        assert!(red.contains("RED — write the failing test first"));
        assert!(
            !red.contains("write the minimal skill"),
            "red must not leak green"
        );
        assert!(green.contains("write the minimal skill"));
        assert!(
            !green.contains("Common Rationalizations for Skipping Testing"),
            "green must not leak rationalizations"
        );
    }

    #[test]
    fn writing_skills_testing_companion_carries_skill_md_sections() {
        let jail = std::path::PathBuf::from(".");
        let types = render_companion(
            "lmd-writing-skills",
            "testing/skill-types",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        assert!(
            types.contains("Testing All Skill Types"),
            "skill-types companion must carry the 'Testing All Skill Types' section (fidelity)"
        );
        let checklist = render_companion(
            "lmd-writing-skills",
            "testing/creation-checklist",
            None,
            None,
            jail,
        )
        .unwrap();
        assert!(
            checklist.contains("Skill Creation Checklist (TDD Adapted)"),
            "creation-checklist companion must carry the 'Skill Creation Checklist' section (fidelity)"
        );
    }

    #[test]
    fn writing_skills_fidelity_all_surfaces_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in ["red", "green", "refactor", "rationalizations"] {
            let out =
                render_skill("lmd-writing-skills", Some(p), None, None, jail.clone()).unwrap();
            assert!(
                out.trim().len() > 80,
                "phase {p} rendered too thin — content lost?"
            );
        }
        for c in [
            "skill-anatomy",
            "skill-discovery-optimization",
            "bulletproofing",
            "testing/methodology",
            "testing/skill-types",
            "testing/creation-checklist",
            "claude-md-testing-example",
            "flowchart-conventions",
            "anthropic-best-practices",
            "persuasion-principles",
        ] {
            let out = render_companion("lmd-writing-skills", c, None, None, jail.clone()).unwrap();
            assert!(
                out.trim().len() > 80,
                "companion {c} rendered too thin — content lost?"
            );
        }
    }

    #[test]
    fn green_phase_renders_tester_dispatch_block() {
        let out = render_skill(
            "lmd-writing-skills",
            Some("green"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        // @dispatch materialised: contract + methodology marker + Iron Law + bootstrap.
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(
            out.contains("RED Phase"),
            "methodology brief missing: {out}"
        );
        assert!(
            out.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "Iron Law via @include missing: {out}"
        );
        assert!(out.contains("role=test"), "test role missing: {out}");
        // to_agent placeholder kept fillable (M-2 guard injects it literally).
        assert!(
            out.contains("to_agent={{ controller_id }}"),
            "controller_id placeholder must survive verbatim: {out}"
        );
        // Phase isolation: refactor's re-dispatch hint must NOT leak into green.
        assert!(
            !out.contains("re-dispatch the same tester"),
            "refactor content leaked into green: {out}"
        );
    }

    #[test]
    fn writing_skills_testing_split_carries_all_original_sections() {
        let jail = std::path::PathBuf::from(".");
        let methodology = render_companion(
            "lmd-writing-skills",
            "testing/methodology",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        // Methodology marker + Iron Law via @include skill-authoring-core.
        assert!(
            methodology.contains("RED Phase: Baseline Testing"),
            "methodology must carry the RED-baseline section: {methodology}"
        );
        assert!(
            methodology.contains("NO SKILL WITHOUT A FAILING TEST FIRST"),
            "methodology must @include skill-authoring-core (Iron Law)"
        );
        let types = render_companion(
            "lmd-writing-skills",
            "testing/skill-types",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        assert!(types.contains("Reference Skills"), "skill-types fidelity");
        let checklist = render_companion(
            "lmd-writing-skills",
            "testing/creation-checklist",
            None,
            None,
            jail,
        )
        .unwrap();
        assert!(
            checklist.contains("Deployment"),
            "creation-checklist fidelity (Deployment section)"
        );
    }

    #[test]
    fn brainstorm_fidelity_all_surfaces_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in [
            "pre-context",
            "explore",
            "questions",
            "approaches",
            "present-design",
            "write-spec",
            "self-review",
            "handoff",
        ] {
            let out = render_skill("lmd-brainstorm", Some(p), None, None, jail.clone())
                .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(
                out.trim().len() > 80,
                "phase {p} rendered too thin — content lost?"
            );
        }
        for c in ["spec-reviewer", "visual-companion"] {
            let out = render_companion("lmd-brainstorm", c, None, None, jail.clone())
                .unwrap_or_else(|_| panic!("companion {c} failed to render"));
            assert!(
                out.trim().len() > 80,
                "companion {c} rendered too thin — content lost?"
            );
        }
    }

    #[test]
    fn brainstorm_stub_description_carries_must_trigger() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let stub = std::fs::read_to_string(
            std::path::Path::new(manifest).join("content/skills/lmd-brainstorm/SKILL.md"),
        )
        .unwrap();
        assert!(
            stub.contains("You MUST use this before any creative work"),
            "stub description must carry the original MUST auto-trigger (spec §SKILL.md-Stub)"
        );
    }

    /// Every installable stub except the bootstrap skill itself must be free of the
    /// `ctx_md_render` handle — it is single-sourced in `lmd-rendering-skills`.
    #[test]
    fn no_installable_stub_mentions_ctx_md_render() {
        let jail = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for name in crate::skill_install::INSTALLABLE_SKILLS {
            if *name == "lmd-rendering-skills" {
                continue;
            }
            let stub =
                crate::skill_source::read_skill_file(&format!("{name}/SKILL.md"), &jail).unwrap();
            assert!(
                !stub.contains("ctx_md_render"),
                "stub {name} still carries the ctx_md_render handle — it belongs \
                 exclusively in lmd-rendering-skills"
            );
        }
    }

    /// …and each of them must point at the skill that does carry it.
    #[test]
    fn every_installable_stub_points_at_lmd_rendering_skills() {
        let jail = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for name in crate::skill_install::INSTALLABLE_SKILLS {
            if *name == "lmd-rendering-skills" {
                continue;
            }
            let stub =
                crate::skill_source::read_skill_file(&format!("{name}/SKILL.md"), &jail).unwrap();
            assert!(
                stub.contains("lmd-rendering-skills"),
                "stub {name} must point at the lmd-rendering-skills skill for \
                 rendering, diagnosis and fallback"
            );
        }
    }

    #[test]
    fn brainstorm_seeds_reference_closure() {
        // No (case-insensitive) `superpowers` token survives in any brainstorm seed.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let base = std::path::Path::new(manifest).join("content/skills/lmd-brainstorm");
        let seeds = [
            "SKILL.md",
            "body.lmd.md",
            "_includes/brainstorm-gate.lmd.md",
            "companions/spec-reviewer.lmd.md",
            "companions/visual-companion.lmd.md",
        ];
        for s in seeds {
            let txt = std::fs::read_to_string(base.join(s)).unwrap();
            assert!(
                !txt.to_lowercase().contains("superpowers"),
                "seed {s} still references superpowers"
            );
        }
    }

    #[test]
    fn file_phase_render() {
        // A 2-phase source; rendering one phase must not leak the other.
        let src = "\
@lean-md
consumer: ai

@phase \"task-1\"
FIRST TASK BODY
@phase-end
@phase \"task-2\"
SECOND TASK BODY
@phase-end
";
        let jail = std::path::PathBuf::from(".");
        let out = render_source_with_phase(src, Some("task-1"), None, None, jail.clone()).unwrap();
        assert!(out.contains("FIRST TASK BODY"), "own phase missing: {out}");
        assert!(!out.contains("SECOND TASK BODY"), "cross-phase leak: {out}");

        // Unknown phase → PhaseNotFound.
        let err = render_source_with_phase(src, Some("task-9"), None, None, jail).unwrap_err();
        assert!(matches!(err, SkillRenderError::PhaseNotFound(_)));
    }

    #[test]
    fn file_phase_vars_prepass() {
        // vars.toml under jail_root/.lean-ctx/lean-md/ overrides an inline @var default,
        // and the override is visible in the rendered phase (proves the file path runs
        // the same prepass as render_skill). jail_root = a temp dir (project root), not
        // the source file's parent.
        let root = std::env::temp_dir().join(format!("lmd_fpr_vars_{}", std::process::id()));
        let vars_dir = root.join(".lean-ctx/lean-md");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&vars_dir).unwrap();
        std::fs::write(
            vars_dir.join("vars.toml"),
            "test_cmd = \"cargo nextest run\"\n",
        )
        .unwrap();

        let src = "\
@lean-md
consumer: ai

@var test_cmd default=\"cargo test\"
@phase \"task-1\"
Run: {{ var test_cmd }} demo
@phase-end
";
        let out = render_source_with_phase(src, Some("task-1"), None, None, root.clone()).unwrap();
        assert!(
            out.contains("cargo nextest run demo"),
            "vars.toml override not applied: {out}"
        );
        assert!(
            !out.contains("cargo test demo"),
            "inline default leaked past vars.toml: {out}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_reviewer_companion_render() {
        // Registry lookup: companion is registered.
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = companion_source("lmd-writing-plans", "plan-reviewer", &jail)
            .expect("plan-reviewer companion must be registered");
        assert!(body.contains("You are a plan document reviewer"));
        assert!(
            !body.to_lowercase().contains("superpowers"),
            "reference-closure: no superpowers token"
        );

        // Render pipeline: non-empty.
        let jail = std::path::PathBuf::from(".");
        let out = render_companion("lmd-writing-plans", "plan-reviewer", None, None, jail).unwrap();
        assert!(!out.trim().is_empty(), "companion must render non-empty");
    }

    #[test]
    fn writing_plans_all_phases_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in [
            "pre-context",
            "file-structure",
            "task-sizing",
            "plan-format",
            "write-plan",
            "self-review",
            "handoff",
        ] {
            let out = render_skill("lmd-writing-plans", Some(p), None, None, jail.clone())
                .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(!out.trim().is_empty(), "phase {p} must render non-empty");
        }
        assert!(
            skill_source("lmd-writing-plans", &jail).is_ok(),
            "lmd-writing-plans must be in the SKILLS registry"
        );
    }

    #[test]
    fn writing_plans_phase_isolation() {
        let jail = std::path::PathBuf::from(".");
        // (phase, own marker present, foreign marker absent)
        let cases = [
            ("file-structure", "map out which files", "Execution Handoff"),
            (
                "task-sizing",
                "Bite-Sized Task Granularity",
                "Writing the plan",
            ),
            (
                "plan-format",
                "No-loss rule inside a task",
                "Execution Handoff",
            ),
            (
                "handoff",
                "Execution Handoff",
                "Bite-Sized Task Granularity",
            ),
        ];
        for (phase, own, foreign) in cases {
            let out =
                render_skill("lmd-writing-plans", Some(phase), None, None, jail.clone()).unwrap();
            assert!(out.contains(own), "phase {phase} missing own marker: {out}");
            assert!(
                !out.contains(foreign),
                "cross-phase leak in {phase}: {foreign}"
            );
        }
    }

    #[test]
    fn writing_plans_body_weaves_code_intel() {
        // §5: file-structure teaches @graph/@impact/@find/@recall for decomposition;
        // plan-format routes verification through @call verify / review_change.
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = skill_source("lmd-writing-plans", &jail).unwrap();
        assert!(
            body.contains("@graph")
                && body.contains("@impact")
                && body.contains("@find")
                && body.contains("@recall"),
            "file-structure must weave the code-intel authoring directives"
        );
        assert!(
            body.contains("@call verify") && body.contains("@call review_change"),
            "plan-format must route verification through recipes"
        );
    }

    #[test]
    fn writing_plans_teaches_crp_compact_and_no_repeat() {
        // Terseness rework: plan-format teaches the crp: compact convention and the
        // "avoid repeating ambient context" output rule; bite-sized routes the standard
        // cycle through recipes.
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = skill_source("lmd-writing-plans", &jail).unwrap();
        assert!(
            body.contains("crp: compact"),
            "plan-format must name the crp: compact convention"
        );
        assert!(
            body.contains("avoid repeating ambient context"),
            "plan-format must teach output_rule #2 (no ambient repetition)"
        );
        assert!(
            body.contains("@call gate") && body.contains("@call tdd"),
            "bite-sized must express the standard cycle as recipe @calls"
        );
    }

    #[test]
    fn dispatch_plan_reviewer_composes() {
        // Rendering self-review executes the @dispatch: the composed output must carry
        // (a) the auto-prepended contract (hard-rules marker) and (b) the plan-reviewer
        // brief, for role=review.
        let jail = std::path::PathBuf::from(".");
        let out = render_skill("lmd-writing-plans", Some("self-review"), None, None, jail).unwrap();
        assert!(
            out.contains("You are a plan document reviewer"),
            "reviewer brief missing: {out}"
        );
        assert!(
            out.contains("lean-ctx"),
            "auto-prepended contract (hard-rules) missing: {out}"
        );
    }

    #[test]
    fn file_phase_macro_prepass() {
        // A macro @define'd in the meta-head (outside the phases) must be visible to a
        // @call inside the isolated task-phase — the macro-prepass warms the registry.
        let src = "\
@lean-md
consumer: ai

@define greet(who)
Hello {{ who }}!
@define-end
@phase \"task-1\"
@call greet(world) /
@phase-end
";
        let jail = std::path::PathBuf::from(".");
        let out = render_source_with_phase(src, Some("task-1"), None, None, jail).unwrap();
        assert!(
            out.contains("Hello world!"),
            "meta-head macro did not expand in phase: {out}"
        );
        assert!(
            !out.contains("@call greet"),
            "@call was not expanded: {out}"
        );
    }

    #[test]
    fn reference_closure_grep() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let files = [
            "content/skills/lmd-writing-plans/body.lmd.md",
            "content/skills/lmd-writing-plans/SKILL.md",
            "content/skills/lmd-writing-plans/companions/plan-reviewer.lmd.md",
            "content/templates/plan-recipes.lmd.md",
            "content/templates/plan-template.lmd.md",
        ];
        for f in files {
            let text = std::fs::read_to_string(std::path::Path::new(manifest).join(f)).unwrap();
            assert!(
                !text.to_lowercase().contains("superpowers"),
                "{f} still references superpowers"
            );
        }
    }

    #[test]
    fn sdd_all_phases_render_nonempty() {
        for phase in [
            "orient",
            "preflight",
            "dispatch",
            "review",
            "final-review",
            "handoff",
        ] {
            let out = render_skill(
                "lmd-subagent-driven-development",
                Some(phase),
                Some(Consumer::Ai),
                None,
                std::env::temp_dir(),
            )
            .unwrap_or_else(|e| panic!("phase {phase} failed: {e}"));
            assert!(!out.trim().is_empty(), "phase {phase} rendered empty");
        }
    }

    #[test]
    fn sdd_phase_isolation_no_cross_phase_leak() {
        let orient = render_skill(
            "lmd-subagent-driven-development",
            Some("orient"),
            Some(Consumer::Ai),
            None,
            std::env::temp_dir(),
        )
        .unwrap();
        // The final-review-only marker must not leak into orient.
        assert!(
            !orient.contains("code-reviewer"),
            "cross-phase leak: {orient}"
        );
    }

    #[test]
    fn sdd_render_is_byte_stable() {
        let a = render_skill(
            "lmd-subagent-driven-development",
            Some("dispatch"),
            Some(Consumer::Ai),
            None,
            std::env::temp_dir(),
        )
        .unwrap();
        let b = render_skill(
            "lmd-subagent-driven-development",
            Some("dispatch"),
            Some(Consumer::Ai),
            None,
            std::env::temp_dir(),
        )
        .unwrap();
        assert_eq!(a, b, "SDD render must be byte-stable (#498)");
    }

    #[test]
    fn sdd_companions_resolve() {
        for c in ["implementer", "task-reviewer", "code-reviewer"] {
            let out = render_companion(
                "lmd-subagent-driven-development",
                c,
                Some(Consumer::Ai),
                None,
                std::env::temp_dir(),
            )
            .unwrap_or_else(|e| panic!("companion {c} failed: {e}"));
            assert!(!out.trim().is_empty(), "companion {c} rendered empty");
        }
    }

    #[test]
    fn sdd_dispatch_implementer_composes() {
        // @dispatch to the implementer prepends the dispatch contract + bootstrap.
        let doc = "@dispatch skill=\"lmd-subagent-driven-development\" companion=\"implementer\" role=dev to_agent=\"c\"\n";
        let out = crate::engine::render(doc);
        assert!(out.contains("Subagent Contract"), "contract missing: {out}");
        assert!(
            out.contains("ToolSearch(query=\"select:mcp__lean-ctx__ctx_read"),
            "bootstrap missing: {out}"
        );
    }

    #[test]
    fn finish_phases_are_rewired_to_lmd_port() {
        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for name in ["lmd-executing-plans", "lmd-subagent-driven-development"] {
            let body = skill_source(name, &jail).unwrap();
            assert!(
                body.contains("lmd-finishing-a-development-branch"),
                "{name} finish phase must invoke the lmd port"
            );
            assert!(
                !body.contains("until an lmd port exists"),
                "{name} must drop the stale external-reference wording"
            );
        }
    }

    #[test]
    fn sdd_dispatch_mode_and_parallel_dispatch_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in ["dispatch-mode", "parallel-dispatch"] {
            let out = render_skill(
                "lmd-subagent-driven-development",
                Some(p),
                None,
                None,
                jail.clone(),
            )
            .unwrap_or_else(|_| panic!("SDD phase {p} failed to render"));
            assert!(
                !out.trim().is_empty(),
                "SDD phase {p} must render non-empty"
            );
        }
    }

    #[test]
    fn sdd_parallel_dispatch_carries_fragment_and_fidelity() {
        let out = render_skill(
            "lmd-subagent-driven-development",
            Some("parallel-dispatch"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("one dispatch per independent problem domain"),
            "parallel-dispatch must resolve @include parallel-dispatch (fragment marker): {out}"
        );
        assert!(
            out.contains("BASE_i") && out.to_lowercase().contains("conflict"),
            "parallel-dispatch must keep per-agent BASE + conflict scan (review fidelity): {out}"
        );
    }
}
