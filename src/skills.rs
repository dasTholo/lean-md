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
const LMD_WS_TESTING_SUBAGENTS: &str = include_str!(
    "../content/skills/lmd-writing-skills/companions/testing-skills-with-subagents.lmd.md"
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
        "testing-skills-with-subagents",
        LMD_WS_TESTING_SUBAGENTS,
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
    fn phase_isolation_no_cross_phase_leak() {
        let out = render_skill(
            "lmd-brainstorm",
            Some("explore"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("EXPLORE_PHASE_MARKER"),
            "explore body missing: {out}"
        );
        assert!(
            !out.contains("HANDOFF_PHASE_MARKER"),
            "cross-phase leak: handoff content rendered: {out}"
        );
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
            "testing-skills-with-subagents",
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
        for n in ["testing-skills-with-subagents", "bulletproofing"] {
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
}
