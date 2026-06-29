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
    let src = skill_body(name).ok_or_else(|| SkillRenderError::UnknownSkill(name.to_string()))?;
    let (mut header, body) = parse_header(src);
    if let Some(c) = consumer {
        header.consumer = c;
    }
    if let Some(m) = crp {
        header.crp = m;
    }
    let ctx = Rc::new(EngineContext::new(header, jail_root));

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
}
