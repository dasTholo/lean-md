//! Skill materialization (Spec §4.6, E7/E11). Writes the thin `SKILL.md` stub
//! (Discovery channel) into `.claude/skills/<name>/`. The heavy body never
//! lands here — it flows through `ctx_md_render` (embedded or `.lean-ctx/lean-md/`
//! overlay). Install home moved into lean-md (Baseline §2.2: lean-ctx installer
//! removed). Opt-in = invocation; `--global|--local` selects the target only.

use std::path::{Path, PathBuf};

const TDD_SKILL_MD: &str = include_str!("../content/skills/lmd-test-driven-development/SKILL.md");
const BRAINSTORM_SKILL_MD: &str = include_str!("../content/skills/lmd-brainstorm/SKILL.md");
const WRITING_SKILLS_SKILL_MD: &str = include_str!("../content/skills/lmd-writing-skills/SKILL.md");

/// Installable lmd skills (name → embedded `SKILL.md` stub).
pub const INSTALLABLE_SKILLS: &[(&str, &str)] = &[
    ("lmd-test-driven-development", TDD_SKILL_MD),
    ("lmd-brainstorm", BRAINSTORM_SKILL_MD),
    ("lmd-writing-skills", WRITING_SKILLS_SKILL_MD),
];

const WRITING_SKILLS_RENDER_GRAPHS: &str =
    include_str!("../content/skills/lmd-writing-skills/render-graphs.js");

/// Non-rendered helper files materialized verbatim into the installed skill dir
/// (skill, filename, embedded content). Absent-only/idempotent like the SKILL.md
/// stub (#498 byte-stable).
const ASSETS: &[(&str, &str, &str)] = &[(
    "lmd-writing-skills",
    "render-graphs.js",
    WRITING_SKILLS_RENDER_GRAPHS,
)];

/// Install target selector (Spec E11). `Local` is the default — env-independent,
/// versionable, team-shareable. `Global` honors `CLAUDE_CONFIG_DIR`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scope {
    Local,
    Global,
}

/// Global Claude state dir (Spec E11/R3): `CLAUDE_CONFIG_DIR` else `~/.claude`.
/// ONLY the global target reacts to `CLAUDE_CONFIG_DIR`.
pub fn claude_state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude")
}

fn skill_md(name: &str) -> Option<&'static str> {
    INSTALLABLE_SKILLS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
}

/// Target dir for a skill under the chosen scope. `--local` is project-relative
/// (env-independent); `--global` is under `claude_state_dir()`.
fn target_dir(name: &str, scope: Scope, project_root: &Path) -> PathBuf {
    match scope {
        Scope::Local => project_root.join(".claude/skills").join(name),
        Scope::Global => claude_state_dir().join("skills").join(name),
    }
}

/// Materialize a skill's `SKILL.md` into the chosen scope. Atomic-ish,
/// idempotent (overwrites the stub — byte-stable content, #498).
pub fn install_skill(name: &str, scope: Scope, project_root: &Path) -> std::io::Result<PathBuf> {
    let body = skill_md(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("unknown installable skill: {name}"),
        )
    })?;
    let dir = target_dir(name, scope, project_root);
    std::fs::create_dir_all(&dir)?;
    let target = dir.join("SKILL.md");
    std::fs::write(&target, body)?;
    for (skill, fname, content) in ASSETS {
        if *skill == name {
            std::fs::write(dir.join(fname), content)?;
        }
    }
    Ok(target)
}

/// Remove the lmd-owned skill dir in the chosen scope only. Absent-tolerant.
pub fn remove_skill(name: &str, scope: Scope, project_root: &Path) -> std::io::Result<()> {
    let dir = target_dir(name, scope, project_root);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_state_dir_honors_config_dir() {
        // SAFETY: single-threaded nextest process-per-test isolates env mutation.
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/pinned-claude") };
        assert_eq!(claude_state_dir(), PathBuf::from("/tmp/pinned-claude"));
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
    }

    #[test]
    fn local_install_is_project_relative_and_ignores_config_dir() {
        let root = std::env::temp_dir().join(format!("lmd_install_local_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // A set CLAUDE_CONFIG_DIR must NOT affect the local target.
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/should-be-ignored") };
        let target = install_skill("lmd-test-driven-development", Scope::Local, &root).unwrap();
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
        let expected = root.join(".claude/skills/lmd-test-driven-development/SKILL.md");
        assert_eq!(target, expected, "local target must be project-relative");
        assert!(target.exists(), "SKILL.md must be written");
        let body = std::fs::read_to_string(&target).unwrap();
        assert!(body.contains("name: lmd-test-driven-development"));
        // Idempotent: second install is fine, file still present.
        install_skill("lmd-test-driven-development", Scope::Local, &root).unwrap();
        assert!(target.exists());
        // Remove takes it away.
        remove_skill("lmd-test-driven-development", Scope::Local, &root).unwrap();
        assert!(!target.exists(), "remove must delete the skill dir");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn global_install_uses_pinned_config_dir() {
        let pin = std::env::temp_dir().join(format!("lmd_install_global_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pin);
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", pin.to_str().unwrap()) };
        let project = std::env::temp_dir().join("lmd_install_global_proj");
        let target = install_skill("lmd-test-driven-development", Scope::Global, &project).unwrap();
        let expected = pin.join("skills/lmd-test-driven-development/SKILL.md");
        assert_eq!(
            target, expected,
            "global target must be under CLAUDE_CONFIG_DIR"
        );
        assert!(target.exists());
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
        let _ = std::fs::remove_dir_all(&pin);
    }

    #[test]
    fn unknown_skill_install_errors() {
        let root = std::env::temp_dir();
        let err = install_skill("nope", Scope::Local, &root).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn writing_skills_install_materializes_asset() {
        let root = std::env::temp_dir().join(format!("lmd_ws_asset_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill("lmd-writing-skills", Scope::Local, &root).unwrap();
        let dir = skill_md.parent().unwrap();
        let asset = dir.join("render-graphs.js");
        assert!(
            asset.exists(),
            "render-graphs.js must be materialized next to SKILL.md"
        );
        let on_disk = std::fs::read_to_string(&asset).unwrap();
        assert!(
            on_disk.contains("extractDotBlocks"),
            "asset content must be the render script"
        );
        // Idempotent: second install keeps the asset present.
        install_skill("lmd-writing-skills", Scope::Local, &root).unwrap();
        assert!(asset.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
