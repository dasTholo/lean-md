//! Skill-content resolution (#727): overlay → pack store → debug fallback.
//!
//! Since P3 the skill bodies, companions, `SKILL.md` stubs, assets and the
//! skill-local `_includes/` live in the `kind=skills` pack
//! `@dastholo/lean-md-skills`, not in this binary. lean-ctx materializes the
//! pack and hands its absolute directory over in one environment variable —
//! this module is the only place that knows about it.
//!
//! Cross-skill core primitives (`hard-rules`, `dispatch-contract`,
//! `parallel-dispatch`) and `gloss/directives` stay `include_str!`-embedded:
//! a general `.lmd.md` render must work in every distribution path, with or
//! without a pack.

use std::path::{Path, PathBuf};

/// Absolute `skills_dir` of the materialized pack. lean-ctx expands it from the
/// `{pack_dir:@dastholo/lean-md-skills}` placeholder in `[mcp.env]` at wiring
/// time (`core/addons/pack_env.rs`). lean-md never derives the store layout.
pub const SKILLS_DIR_ENV: &str = "LEAN_MD_SKILLS_DIR";

#[derive(Debug)]
pub enum SourceError {
    /// No content root at all — production install is broken. Actionable, never silent.
    PackMissing(String),
    /// A content root exists, but it does not carry this relative path.
    NotFound(String),
    Io(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::PackMissing(m) => write!(f, "PACK_MISSING {m}"),
            SourceError::NotFound(p) => write!(f, "SKILL_FILE_NOT_FOUND '{p}'"),
            SourceError::Io(e) => write!(f, "SKILL_FILE_IO {e}"),
        }
    }
}

/// The materialized pack root, if lean-ctx wired one and it exists on disk.
pub fn pack_store_root() -> Option<PathBuf> {
    let raw = std::env::var(SKILLS_DIR_ENV).ok()?;
    if raw.is_empty() {
        return None;
    }
    let root = PathBuf::from(raw);
    root.is_dir().then_some(root)
}

/// Dev-only content root: `$CARGO_MANIFEST_DIR/content/skills`. Inert in a release
/// binary — `debug_assertions` is off there and the path does not exist on a user's
/// machine either. Both guards must fail for production to reach `PackMissing`.
pub fn debug_fallback_root() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("content/skills");
    root.is_dir().then_some(root)
}

/// Jailed project overlay root — local phase iteration without a pack republish.
fn overlay_root(jail_root: &Path) -> PathBuf {
    jail_root.join(".lean-ctx/lean-md/skills")
}

/// Builds the `PackMissing` message, naming `SKILLS_DIR_ENV` either as unset or
/// as set-but-invalid. Split out of `content_root()` so it is directly
/// unit-testable — the debug fallback (`content/skills` exists in this dev
/// checkout) otherwise masks this arm in every `cargo nextest run`.
fn pack_missing_message() -> String {
    match std::env::var(SKILLS_DIR_ENV) {
        Ok(raw) if !raw.is_empty() => format!(
            "{SKILLS_DIR_ENV}={raw} is not a directory — reinstall the addon: \
             `lean-ctx addon add @dastholo/lean-md`"
        ),
        _ => format!(
            "{SKILLS_DIR_ENV} is unset — the skills pack was never wired. \
             Reinstall the addon: `lean-ctx addon add @dastholo/lean-md`"
        ),
    }
}

fn content_root() -> Result<PathBuf, SourceError> {
    if let Some(pack) = pack_store_root() {
        return Ok(pack);
    }
    if let Some(dev) = debug_fallback_root() {
        return Ok(dev);
    }
    Err(SourceError::PackMissing(pack_missing_message()))
}

/// Read one skill-content file by its pack-relative path (e.g.
/// `lmd-brainstorm/body.lmd.md`) through the three-stage cascade.
///
/// Stage 1 is PathJail-bound: an overlay may never reach outside `jail_root`.
/// Stages 2 and 3 take `rel` from a static registry, never from user input.
pub fn read_skill_file(rel: &str, jail_root: &Path) -> Result<String, SourceError> {
    let overlay = overlay_root(jail_root).join(rel);
    if let Ok(resolved) = crate::pathx::jail_path(&overlay, jail_root)
        && resolved.is_file()
    {
        return std::fs::read_to_string(&resolved).map_err(|e| SourceError::Io(e.to_string()));
    }
    let candidate = content_root()?.join(rel);
    if !candidate.is_file() {
        return Err(SourceError::NotFound(rel.to_string()));
    }
    std::fs::read_to_string(&candidate).map_err(|e| SourceError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("skill_source_{name}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn overlay_wins_over_pack_store() {
        let jail_root = temp_dir("overlay_wins_jail");
        let pack_root = temp_dir("overlay_wins_pack");
        let rel = "lmd-demo/body.lmd.md";

        std::fs::create_dir_all(pack_root.join("lmd-demo")).unwrap();
        std::fs::write(pack_root.join(rel), "from pack").unwrap();

        let overlay_file = jail_root.join(".lean-ctx/lean-md/skills").join(rel);
        std::fs::create_dir_all(overlay_file.parent().unwrap()).unwrap();
        std::fs::write(&overlay_file, "from overlay").unwrap();

        crate::test_env::set_var(SKILLS_DIR_ENV, pack_root.to_str().unwrap());
        let got = read_skill_file(rel, &jail_root).unwrap();
        crate::test_env::remove_var(SKILLS_DIR_ENV);

        assert_eq!(got, "from overlay");
    }

    #[test]
    fn pack_store_resolves_when_no_overlay_exists() {
        let jail_root = temp_dir("pack_only_jail");
        let pack_root = temp_dir("pack_only_pack");
        let rel = "lmd-demo/body.lmd.md";

        std::fs::create_dir_all(pack_root.join("lmd-demo")).unwrap();
        std::fs::write(pack_root.join(rel), "from pack").unwrap();

        crate::test_env::set_var(SKILLS_DIR_ENV, pack_root.to_str().unwrap());
        let got = read_skill_file(rel, &jail_root).unwrap();
        crate::test_env::remove_var(SKILLS_DIR_ENV);

        assert_eq!(got, "from pack");
    }

    #[test]
    fn missing_file_inside_existing_pack_root_is_not_found_not_pack_missing() {
        let jail_root = temp_dir("notfound_jail");
        let pack_root = temp_dir("notfound_pack");

        crate::test_env::set_var(SKILLS_DIR_ENV, pack_root.to_str().unwrap());
        let err = read_skill_file("lmd-nope/body.lmd.md", &jail_root).unwrap_err();
        crate::test_env::remove_var(SKILLS_DIR_ENV);

        match err {
            SourceError::NotFound(rel) => assert_eq!(rel, "lmd-nope/body.lmd.md"),
            other => panic!("expected NotFound, got {other}"),
        }
    }

    // NOTE (#727): `content_root()`'s debug-fallback branch (`content/skills`
    // inside this checkout) always succeeds under `cargo nextest run` — debug
    // build, `content/skills` exists on disk — so an unset/bogus
    // LEAN_MD_SKILLS_DIR can never actually surface `PackMissing` end-to-end
    // through `read_skill_file()` in this dev/test environment; only a real
    // release binary without a wired pack (and without `content/skills`)
    // reaches that arm. `cfg!(debug_assertions)` and `env!("CARGO_MANIFEST_DIR")`
    // are both compile-time constants — no test-runtime env mutation can flip
    // them, and moving the real `content/skills` dir during a test would race
    // every other test in this suite that reads it concurrently (nextest runs
    // tests in parallel processes; see fragments.rs/gloss.rs/phases.rs/audit.rs).
    // We test the `PackMissing` message construction directly instead of
    // pretending to reproduce a release build.
    #[test]
    fn pack_missing_message_when_env_unset_names_reinstall_hint() {
        crate::test_env::remove_var(SKILLS_DIR_ENV);
        let msg = pack_missing_message();
        assert!(msg.contains("is unset"), "got: {msg}");
        assert!(msg.contains(SKILLS_DIR_ENV), "got: {msg}");
        assert!(msg.contains("lean-ctx addon add"), "got: {msg}");
    }

    #[test]
    fn pack_missing_message_when_env_set_but_missing_names_the_var() {
        let bogus = std::env::temp_dir().join("skill_source_does_not_exist_xyz");
        crate::test_env::set_var(SKILLS_DIR_ENV, bogus.to_str().unwrap());
        let msg = pack_missing_message();
        crate::test_env::remove_var(SKILLS_DIR_ENV);
        assert!(msg.contains(SKILLS_DIR_ENV), "got: {msg}");
        assert!(msg.contains(bogus.to_str().unwrap()), "got: {msg}");
        assert!(msg.contains("is not a directory"), "got: {msg}");
    }
}
