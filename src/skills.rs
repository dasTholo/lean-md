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

/// Registry of embedded lmd skill bodies (name → binary-embedded body source).
/// Replaces the hardcoded `match` so new skills are a one-line table entry
/// (Spec E4 — companion column deferred to Spec #2).
const SKILLS: &[(&str, &str)] = &[
    ("lmd-brainstorm", LMD_BRAINSTORM_BODY),
    (
        "lmd-test-driven-development",
        LMD_TEST_DRIVEN_DEVELOPMENT_BODY,
    ),
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

#[derive(Debug)]
pub enum SkillRenderError {
    UnknownSkill(String),
    PhaseNotFound(String),
}

impl std::fmt::Display for SkillRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillRenderError::UnknownSkill(s) => write!(f, "UNKNOWN_SKILL '{s}'"),
            SkillRenderError::PhaseNotFound(p) => write!(f, "PHASE_NOT_FOUND '{p}'"),
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
}
