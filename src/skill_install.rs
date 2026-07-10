//! Skill materialization (Spec §4.6, E7/E11). Writes the thin `SKILL.md` stub
//! (Discovery channel) into `.claude/skills/<name>/`. The heavy body never
//! lands here — it flows through `ctx_md_render` (embedded or `.lean-ctx/lean-md/`
//! overlay). Install home moved into lean-md (Baseline §2.2: lean-ctx installer
//! removed). Opt-in = invocation; `--global|--local` selects the target only.

use std::path::{Path, PathBuf};

/// Installable lmd skills. The stub path is derived: `<name>/SKILL.md`.
pub const INSTALLABLE_SKILLS: &[&str] = &[
    "lmd-test-driven-development",
    "lmd-brainstorm",
    "lmd-writing-skills",
    "lmd-writing-plans",
    "lmd-subagent-driven-development",
    "lmd-executing-plans",
    "lmd-finishing-a-development-branch",
    "lmd-dispatching-parallel-agents",
];

/// Non-rendered helper files materialized verbatim into the installed skill dir
/// (skill, pack-relative filename). Read from the content cascade at install time.
const ASSETS: &[(&str, &str)] = &[
    ("lmd-writing-skills", "render-graphs.js"),
    ("lmd-brainstorm", "scripts/server.cjs"),
    ("lmd-brainstorm", "scripts/helper.js"),
    ("lmd-brainstorm", "scripts/frame-template.html"),
    ("lmd-brainstorm", "scripts/start-server.sh"),
    ("lmd-brainstorm", "scripts/stop-server.sh"),
];

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

/// The `SKILL.md` stub of an installable skill, read through the content cascade.
/// `project_root` doubles as the jail root, so a project overlay wins here too.
fn skill_md(name: &str, project_root: &Path) -> std::io::Result<String> {
    if !INSTALLABLE_SKILLS.contains(&name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("unknown installable skill: {name}"),
        ));
    }
    crate::skill_source::read_skill_file(&format!("{name}/SKILL.md"), project_root)
        .map_err(|e| std::io::Error::other(e.to_string()))
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
/// idempotent (overwrites the stub — byte-stable content, #498). `force=true`
/// additionally refreshes the project-level seeds even when they already exist
/// (a stale derived seed after an embedded-seed edit); `force=false` keeps the
/// seeds absent-only so user edits are never clobbered.
pub fn install_skill(
    name: &str,
    scope: Scope,
    project_root: &Path,
    force: bool,
) -> std::io::Result<PathBuf> {
    let body = skill_md(name, project_root)?;
    let assets: Vec<(&str, String)> = ASSETS
        .iter()
        .filter(|(skill, _)| *skill == name)
        .map(|(_, fname)| {
            crate::skill_source::read_skill_file(&format!("{name}/{fname}"), project_root)
                .map(|content| (*fname, content))
                .map_err(|e| std::io::Error::other(e.to_string()))
        })
        .collect::<std::io::Result<_>>()?;
    let dir = target_dir(name, scope, project_root);
    std::fs::create_dir_all(&dir)?;
    let target = dir.join("SKILL.md");
    std::fs::write(&target, body)?;
    let mut created_parents: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::new();
    for (fname, content) in &assets {
        let asset_path = dir.join(fname);
        if let Some(parent) = asset_path.parent()
            && created_parents.insert(parent.to_path_buf())
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&asset_path, content)?;
        #[cfg(unix)]
        if fname.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(&asset_path)?.permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&asset_path, perm)?;
        }
    }
    // Materialize the project-level seeds (plan-recipes/plan-template, lang/*,
    // tooling/*, dispatch-contract.ext) into the project root — absent-only unless
    // `force`, so user edits are never overwritten by a plain reinstall (Spec §6);
    // `--force` refreshes a stale derived seed after an embedded-seed edit.
    crate::seeds::materialize_contracts(project_root, ".lean-ctx/lean-md", force)?;
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
        let target =
            install_skill("lmd-test-driven-development", Scope::Local, &root, false).unwrap();
        unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") };
        let expected = root.join(".claude/skills/lmd-test-driven-development/SKILL.md");
        assert_eq!(target, expected, "local target must be project-relative");
        assert!(target.exists(), "SKILL.md must be written");
        let body = std::fs::read_to_string(&target).unwrap();
        assert!(body.contains("name: lmd-test-driven-development"));
        // Idempotent: second install is fine, file still present.
        install_skill("lmd-test-driven-development", Scope::Local, &root, false).unwrap();
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
        let target = install_skill(
            "lmd-test-driven-development",
            Scope::Global,
            &project,
            false,
        )
        .unwrap();
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
        let err = install_skill("nope", Scope::Local, &root, false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn unknown_skill_install_errors_before_touching_filesystem() {
        let root = std::env::temp_dir().join(format!("lmd_unknown_notouch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let err = install_skill("nope", Scope::Local, &root, false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert!(
            !root.join(".claude/skills/nope").exists(),
            "an unknown skill must not create a target dir"
        );
        assert!(
            !root.join(".claude").exists(),
            "an unknown skill must fail before touching the filesystem at all"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn install_skill_reads_skill_md_from_content_cascade() {
        // The project overlay is stage 1 of the cascade (skill_source.rs) — proving it
        // wins here proves `install_skill` actually goes through `read_skill_file`
        // rather than any statically embedded content.
        let root = std::env::temp_dir().join(format!("lmd_cascade_stub_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let overlay_dir = root.join(".lean-ctx/lean-md/skills/lmd-test-driven-development");
        std::fs::create_dir_all(&overlay_dir).unwrap();
        std::fs::write(
            overlay_dir.join("SKILL.md"),
            "---\nname: lmd-test-driven-development\n---\nOVERLAY MARKER 8f3c\n",
        )
        .unwrap();

        let target =
            install_skill("lmd-test-driven-development", Scope::Local, &root, false).unwrap();
        let written = std::fs::read_to_string(&target).unwrap();
        assert!(
            written.contains("OVERLAY MARKER 8f3c"),
            "install_skill must read SKILL.md through the content cascade (overlay wins): {written}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn install_skill_surfaces_cascade_read_failure_as_io_error_without_empty_stub() {
        // `SourceError::PackMissing` itself is only reachable from a release binary
        // without a wired pack — skill_source.rs's own tests document that the debug
        // fallback (`content/skills` in this checkout) always succeeds under
        // `cargo nextest run`, masking that arm. Pointing LEAN_MD_SKILLS_DIR at a
        // valid-but-empty directory exercises the identical conversion path
        // (`skill_md`'s `.map_err(|e| io::Error::other(...))`) through the variant
        // that IS reachable in-process (`SourceError::NotFound`), proving errors
        // surface as an `io::Error` instead of a silently-written empty stub.
        let root = std::env::temp_dir().join(format!("lmd_cascade_fail_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let empty_pack =
            std::env::temp_dir().join(format!("lmd_empty_pack_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&empty_pack);
        std::fs::create_dir_all(&empty_pack).unwrap();

        unsafe { std::env::set_var("LEAN_MD_SKILLS_DIR", &empty_pack) };
        let err =
            install_skill("lmd-test-driven-development", Scope::Local, &root, false).unwrap_err();
        unsafe { std::env::remove_var("LEAN_MD_SKILLS_DIR") };

        assert_eq!(
            err.kind(),
            std::io::ErrorKind::Other,
            "a cascade read failure must surface via io::Error::other (distinct from the \
             unknown-skill NotFound gate), got: {err:?}"
        );
        assert!(
            !err.to_string().is_empty(),
            "error message must not be empty"
        );
        assert!(
            !root
                .join(".claude/skills/lmd-test-driven-development")
                .exists(),
            "a cascade read failure must not leave a half-written/empty stub on disk"
        );

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&empty_pack);
    }

    #[test]
    fn brainstorm_install_materializes_scripts() {
        let root = std::env::temp_dir().join(format!("lmd_bs_assets_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let skill_md = install_skill("lmd-brainstorm", Scope::Local, &root, false).unwrap();
        let dir = skill_md.parent().unwrap();
        let scripts = dir.join("scripts");

        for f in [
            "server.cjs",
            "helper.js",
            "frame-template.html",
            "start-server.sh",
            "stop-server.sh",
        ] {
            assert!(
                scripts.join(f).exists(),
                "scripts/{f} must be materialized next to SKILL.md"
            );
        }

        // .sh must be executable (unix).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for sh in ["start-server.sh", "stop-server.sh"] {
                let mode = std::fs::metadata(scripts.join(sh))
                    .unwrap()
                    .permissions()
                    .mode();
                assert!(mode & 0o111 != 0, "scripts/{sh} must be executable");
            }
        }

        // Idempotent: a second install keeps the assets present.
        install_skill("lmd-brainstorm", Scope::Local, &root, false).unwrap();
        assert!(scripts.join("server.cjs").exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn brainstorm_assets_reference_closure() {
        // No (case-insensitive) `superpowers` token survives in any cascade-resolved asset.
        let jail = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (skill, fname) in ASSETS {
            if *skill == "lmd-brainstorm" {
                let content =
                    crate::skill_source::read_skill_file(&format!("{skill}/{fname}"), &jail)
                        .expect("brainstorm asset resolves");
                assert!(
                    !content.to_lowercase().contains("superpowers"),
                    "asset {fname} still references superpowers"
                );
            }
        }
    }

    #[test]
    fn writing_plans_install_writes_skill_md() {
        let root = std::env::temp_dir().join(format!("lmd_wp_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let skill_md = install_skill("lmd-writing-plans", Scope::Local, &root, false).unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-writing-plans"),
            "stub frontmatter missing"
        );
        assert!(
            !written.to_lowercase().contains("superpowers"),
            "reference-closure in stub"
        );

        // Idempotent.
        install_skill("lmd-writing-plans", Scope::Local, &root, false).unwrap();
        assert!(skill_md.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sdd_install_writes_skill_md() {
        let root = std::env::temp_dir().join(format!("lmd_sdd_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill(
            "lmd-subagent-driven-development",
            Scope::Local,
            &root,
            false,
        )
        .unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-subagent-driven-development"),
            "stub frontmatter missing"
        );
        // Guard the YAML frontmatter `description`: it must be a single-line, non-empty
        // scalar — the value on the SAME line as the key (not wrapped onto the next line)
        // and, when unquoted, free of the `": "` mapping indicator that silently turns the
        // scalar into a nested map / hard parse error (regression guard for the invalid
        // frontmatter this fix repaired).
        let desc_line = written
            .lines()
            .find(|l| l.starts_with("description:"))
            .expect("description key missing");
        let value = desc_line["description:".len()..].trim();
        assert!(
            !value.is_empty(),
            "description must be a same-line non-empty scalar, not wrapped/empty: {desc_line:?}"
        );
        let quoted = value.starts_with('"') || value.starts_with('\'');
        assert!(
            quoted || !value.contains(": "),
            "unquoted description scalar must not contain ': ' (YAML mapping indicator): {value}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn executing_plans_install_writes_skill_md() {
        let root =
            std::env::temp_dir().join(format!("lmd_execplans_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill("lmd-executing-plans", Scope::Local, &root, false).unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-executing-plans"),
            "stub frontmatter missing"
        );
        // Reference-closure: a native port must not name the upstream skill source.
        assert!(
            !written.contains("superpowers"),
            "native port must not carry a 'superpowers' reference"
        );
        // Frontmatter-scalar guard: single-line, non-empty description; unquoted must be free
        // of the ': ' YAML mapping indicator (else it silently parses as a nested map).
        let desc_line = written
            .lines()
            .find(|l| l.starts_with("description:"))
            .expect("description key missing");
        let value = desc_line["description:".len()..].trim();
        assert!(
            !value.is_empty(),
            "description must be a non-empty same-line scalar: {desc_line:?}"
        );
        let quoted = value.starts_with('"') || value.starts_with('\'');
        assert!(
            quoted || !value.contains(": "),
            "unquoted description scalar must not contain ': ': {value}"
        );
        // Idempotent: a second install over the existing dir is a no-op success.
        let again = install_skill("lmd-executing-plans", Scope::Local, &root, false).unwrap();
        assert_eq!(again, skill_md, "install must be idempotent");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn install_wires_seeds() {
        let root = std::env::temp_dir().join(format!("lmd_wire_seeds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        install_skill("lmd-writing-plans", Scope::Local, &root, false).unwrap();
        let base = root.join(".lean-ctx/lean-md");
        assert!(
            base.join("plan-recipes.lmd.md").exists(),
            "install must materialize plan-recipes"
        );
        assert!(
            base.join("plan-template.lmd.md").exists(),
            "install must materialize plan-template"
        );

        // User edit is preserved on a second install (absent-only).
        std::fs::write(base.join("plan-recipes.lmd.md"), "# user edit\n").unwrap();
        install_skill("lmd-writing-plans", Scope::Local, &root, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(base.join("plan-recipes.lmd.md")).unwrap(),
            "# user edit\n",
            "second install must not overwrite user edits"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn install_force_refreshes_stale_seed() {
        // M2: `skill install --force` refreshes a stale local seed via install_skill(force=true),
        // where a plain reinstall (absent-only) would leave it stale.
        let root = std::env::temp_dir().join(format!("lmd_force_seed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        install_skill("lmd-writing-plans", Scope::Local, &root, false).unwrap();
        let seed = root.join(".lean-ctx/lean-md/plan-recipes.lmd.md");
        std::fs::write(&seed, "# stale\n").unwrap();

        // force=true rewrites the stale seed back to the embedded content.
        install_skill("lmd-writing-plans", Scope::Local, &root, true).unwrap();
        let refreshed = std::fs::read_to_string(&seed).unwrap();
        assert_ne!(
            refreshed, "# stale\n",
            "force reinstall must refresh the stale seed"
        );
        assert!(
            refreshed.contains("@define gate("),
            "refreshed seed must carry the current embedded recipes"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn finishing_install_writes_skill_md() {
        let root =
            std::env::temp_dir().join(format!("lmd_finishing_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill(
            "lmd-finishing-a-development-branch",
            Scope::Local,
            &root,
            false,
        )
        .unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-finishing-a-development-branch"),
            "stub frontmatter missing"
        );
        assert!(
            !written.contains("superpowers"),
            "native port must not carry a 'superpowers' reference"
        );
        let desc_line = written
            .lines()
            .find(|l| l.starts_with("description:"))
            .expect("description key missing");
        let value = desc_line["description:".len()..].trim();
        assert!(
            !value.is_empty(),
            "description must be a non-empty same-line scalar: {desc_line:?}"
        );
        let quoted = value.starts_with('"') || value.starts_with('\'');
        assert!(
            quoted || !value.contains(": "),
            "unquoted description scalar must not contain ': ': {value}"
        );
        let again = install_skill(
            "lmd-finishing-a-development-branch",
            Scope::Local,
            &root,
            false,
        )
        .unwrap();
        assert_eq!(again, skill_md, "install must be idempotent");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn writing_skills_install_materializes_asset() {
        let root = std::env::temp_dir().join(format!("lmd_ws_asset_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill("lmd-writing-skills", Scope::Local, &root, false).unwrap();
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
        install_skill("lmd-writing-skills", Scope::Local, &root, false).unwrap();
        assert!(asset.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dispatching_parallel_agents_install_writes_skill_md() {
        let root =
            std::env::temp_dir().join(format!("lmd_dispatch_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md = install_skill(
            "lmd-dispatching-parallel-agents",
            Scope::Local,
            &root,
            false,
        )
        .unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-dispatching-parallel-agents"),
            "stub frontmatter missing"
        );
        assert!(
            !written.contains("superpowers"),
            "native port must not carry a 'superpowers' reference"
        );
        let desc_line = written
            .lines()
            .find(|l| l.starts_with("description:"))
            .expect("description key missing");
        let value = desc_line["description:".len()..].trim();
        assert!(
            !value.is_empty(),
            "description must be a non-empty same-line scalar: {desc_line:?}"
        );
        let quoted = value.starts_with('"') || value.starts_with('\'');
        assert!(
            quoted || !value.contains(": "),
            "unquoted description scalar must not contain ': ': {value}"
        );
        let again = install_skill(
            "lmd-dispatching-parallel-agents",
            Scope::Local,
            &root,
            false,
        )
        .unwrap();
        assert_eq!(again, skill_md, "install must be idempotent");
        let _ = std::fs::remove_dir_all(&root);
    }
}
