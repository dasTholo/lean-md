//! Embedded lmd skill bodies + phase-isolated render-on-invoke (Spec §5.4).
//! Bodies ship as `include_str!`-embedded seeds (binary-only, byte-stable #498);
//! `ctx_md_render(skill, phase)` renders one isolated phase against them.

use std::path::PathBuf;
use std::rc::Rc;

use crate::crp_proto::CrpMode;
use crate::engine::{EngineContext, render_body};
use crate::header::{Consumer, parse_header};

const LMD_BRAINSTORM_BODY: &str = include_str!("../content/skills/lmd-brainstorm/body.lmd.md");
const LMD_TEST_DRIVEN_DEVELOPMENT_BODY: &str =
    include_str!("../content/skills/lmd-test-driven-development/body.lmd.md");
const LMD_WRITING_SKILLS_BODY: &str =
    include_str!("../content/skills/lmd-writing-skills/body.lmd.md");
const LMD_TESTING_ANTI_PATTERNS_COMPANION: &str = include_str!(
    "../content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md"
);
const LMD_WS_SKILL_ANATOMY: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/skill-anatomy.lmd.md");
const LMD_WS_SDO: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/skill-discovery-optimization.lmd.md"
);
const LMD_WS_BULLETPROOFING: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/bulletproofing.lmd.md");
const LMD_WS_TESTING_METHODOLOGY: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/testing/methodology.lmd.md");
const LMD_WS_TESTING_SKILL_TYPES: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/testing/skill-types.lmd.md");
const LMD_WS_TESTING_CREATION_CHECKLIST: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/testing/creation-checklist.lmd.md"
);
const LMD_WS_CLAUDE_MD_TESTING: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/claude-md-testing-example.lmd.md"
);
const LMD_WS_FLOWCHART: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/flowchart-conventions.lmd.md");
const LMD_WS_ANTHROPIC_BP: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/anthropic-best-practices.lmd.md");
const LMD_WS_PERSUASION: &str =
    include_str!("../content/skills/lmd-writing-skills/companions/persuasion-principles.lmd.md");
const LMD_BRAINSTORM_SPEC_REVIEWER: &str =
    include_str!("../content/skills/lmd-brainstorm/companions/spec-reviewer.lmd.md");
const LMD_WRITING_PLANS_PLAN_REVIEWER: &str =
    include_str!("../content/skills/lmd-writing-plans/companions/plan-reviewer.lmd.md");
const LMD_BRAINSTORM_VISUAL_COMPANION: &str =
    include_str!("../content/skills/lmd-brainstorm/companions/visual-companion.lmd.md");

/// Registry of embedded lmd skill bodies (name → binary-embedded body source).
/// Replaces the hardcoded `match` so new skills are a one-line table entry
/// (Spec E4 — companion column deferred to Spec #2).
const SKILLS: &[(&str, &str)] = &[
    ("lmd-brainstorm", LMD_BRAINSTORM_BODY),
    (
        "lmd-test-driven-development",
        LMD_TEST_DRIVEN_DEVELOPMENT_BODY,
    ),
    ("lmd-writing-skills", LMD_WRITING_SKILLS_BODY),
];

/// Embedded body source for a known lmd skill, or `None` if unknown.
pub fn skill_body(name: &str) -> Option<&'static str> {
    SKILLS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, body)| *body)
}

/// All embedded skill bodies (for cross-skill `@var` aggregation in `vars --init`).
pub fn all_skill_bodies() -> Vec<&'static str> {
    SKILLS.iter().map(|(_, b)| *b).collect()
}

/// Registry of embedded companions (skill, companion name → embedded body).
/// Out-of-band on-demand references attached to a skill (Spec #2, E1/A).
const COMPANIONS: &[(&str, &str, &str)] = &[
    (
        "lmd-test-driven-development",
        "testing-anti-patterns",
        LMD_TESTING_ANTI_PATTERNS_COMPANION,
    ),
    ("lmd-writing-skills", "skill-anatomy", LMD_WS_SKILL_ANATOMY),
    (
        "lmd-writing-skills",
        "skill-discovery-optimization",
        LMD_WS_SDO,
    ),
    (
        "lmd-writing-skills",
        "bulletproofing",
        LMD_WS_BULLETPROOFING,
    ),
    (
        "lmd-writing-skills",
        "testing/methodology",
        LMD_WS_TESTING_METHODOLOGY,
    ),
    (
        "lmd-writing-skills",
        "testing/skill-types",
        LMD_WS_TESTING_SKILL_TYPES,
    ),
    (
        "lmd-writing-skills",
        "testing/creation-checklist",
        LMD_WS_TESTING_CREATION_CHECKLIST,
    ),
    (
        "lmd-writing-skills",
        "claude-md-testing-example",
        LMD_WS_CLAUDE_MD_TESTING,
    ),
    (
        "lmd-writing-skills",
        "flowchart-conventions",
        LMD_WS_FLOWCHART,
    ),
    (
        "lmd-writing-skills",
        "anthropic-best-practices",
        LMD_WS_ANTHROPIC_BP,
    ),
    (
        "lmd-writing-skills",
        "persuasion-principles",
        LMD_WS_PERSUASION,
    ),
    (
        "lmd-brainstorm",
        "spec-reviewer",
        LMD_BRAINSTORM_SPEC_REVIEWER,
    ),
    (
        "lmd-brainstorm",
        "visual-companion",
        LMD_BRAINSTORM_VISUAL_COMPANION,
    ),
    (
        "lmd-writing-plans",
        "plan-reviewer",
        LMD_WRITING_PLANS_PLAN_REVIEWER,
    ),
];

/// Embedded body for a known `(skill, companion)` pair, or `None` if unknown.
pub fn companion_body(skill: &str, companion: &str) -> Option<&'static str> {
    COMPANIONS
        .iter()
        .find(|(s, c, _)| *s == skill && *c == companion)
        .map(|(_, _, body)| *body)
}

#[derive(Debug)]
pub enum SkillRenderError {
    UnknownSkill(String),
    PhaseNotFound(String),
    CompanionNotFound(String),
}

impl std::fmt::Display for SkillRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillRenderError::UnknownSkill(s) => write!(f, "UNKNOWN_SKILL '{s}'"),
            SkillRenderError::PhaseNotFound(p) => write!(f, "PHASE_NOT_FOUND '{p}'"),
            SkillRenderError::CompanionNotFound(c) => write!(f, "COMPANION_NOT_FOUND '{c}'"),
        }
    }
}

/// D7 body-override: a jailed project overlay at
/// `<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md` wins over the
/// embedded const, enabling local phase iteration without a recompile.
/// PathJail-bound (no escape outside `jail_root`).
fn overlay_body(name: &str, jail_root: &std::path::Path) -> Option<String> {
    let candidate = jail_root
        .join(".lean-ctx/lean-md/skills")
        .join(name)
        .join("body.lmd.md");
    let resolved = crate::pathx::jail_path(&candidate, jail_root).ok()?;
    if !resolved.exists() {
        return None;
    }
    std::fs::read_to_string(&resolved).ok()
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
    let owned_overlay = overlay_body(name, &jail_root);
    let src: &str = match owned_overlay.as_deref() {
        Some(s) => s,
        None => skill_body(name).ok_or_else(|| SkillRenderError::UnknownSkill(name.to_string()))?,
    };
    let (mut header, body) = parse_header(src);
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
    let src = companion_body(skill, companion)
        .ok_or_else(|| SkillRenderError::CompanionNotFound(format!("{skill}/{companion}")))?;
    Ok(render_full_source(src, consumer, crp, jail_root))
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
            skill_body("lmd-brainstorm").is_some(),
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
        assert!(skill_body("lmd-brainstorm").is_some());
        assert!(skill_body("lmd-test-driven-development").is_some());
        assert!(skill_body("nope").is_none());
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
    fn tdd_body_matches_seed_file_on_disk() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let disk = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-test-driven-development/body.lmd.md"),
        )
        .unwrap();
        assert_eq!(
            skill_body("lmd-test-driven-development").unwrap(),
            disk,
            "embedded TDD body drifted from seed file"
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
        let bodies = all_skill_bodies();
        let decls: Vec<_> = bodies
            .iter()
            .flat_map(|b| crate::skill_vars::scan_var_decls(b))
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
        assert!(companion_body("lmd-test-driven-development", "testing-anti-patterns").is_some());
        assert!(companion_body("lmd-test-driven-development", "nope").is_none());
        assert!(companion_body("nope", "testing-anti-patterns").is_none());
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
    fn companion_body_matches_seed_file_on_disk() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let disk = std::fs::read_to_string(std::path::Path::new(manifest).join(
            "content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md",
        ))
        .unwrap();
        assert_eq!(
            companion_body("lmd-test-driven-development", "testing-anti-patterns").unwrap(),
            disk,
            "embedded companion drifted from seed file"
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
        let mut corpus: Vec<&'static str> = all_skill_bodies();
        corpus.extend(COMPANIONS.iter().map(|(_, _, body)| *body));

        for body in corpus {
            // Skill-scoped render calls: companion must resolve under that skill.
            for cap in call_re.captures_iter(body) {
                let skill = &cap[1];
                let companion = &cap[2];
                assert!(
                    companion_body(skill, companion).is_some(),
                    "dangling companion ref: skill=\"{skill}\" companion=\"{companion}\""
                );
            }
            // Every companion mention (prose or call) must name a registered companion.
            for cap in mention_re.captures_iter(body) {
                let name = &cap[1];
                assert!(
                    COMPANIONS.iter().any(|(_, c, _)| *c == name),
                    "dangling companion ref: companion \"{name}\" not in COMPANIONS"
                );
            }
        }
    }

    #[test]
    fn brainstorm_spec_reviewer_resolves() {
        let body = companion_body("lmd-brainstorm", "spec-reviewer")
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
        let body = companion_body("lmd-brainstorm", "visual-companion")
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
        assert!(stub.contains("Where this runs"));
        for call in [
            "phase=\"red\"",
            "phase=\"green\"",
            "phase=\"refactor\"",
            "phase=\"rationalizations\"",
            "companion=\"testing-anti-patterns\"",
        ] {
            assert!(stub.contains(call), "stub missing render call '{call}'");
        }
        // Companion trigger (E7, upstream wording) + final rule + XOR.
        assert!(stub.contains("When adding mocks or test utilities"));
        assert!(stub.contains("never both"));
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
    }

    #[test]
    fn writing_skills_is_registered() {
        assert!(
            skill_body("lmd-writing-skills").is_some(),
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
        for n in names {
            let body = companion_body("lmd-writing-skills", n)
                .unwrap_or_else(|| panic!("companion {n} not registered"));
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
        assert!(
            stub.contains("ctx_md_render"),
            "stub description/body must keep the lmd render-on-invoke pointer"
        );
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
        let body = companion_body("lmd-writing-plans", "plan-reviewer")
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
}
