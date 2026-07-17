//! Project materialization of lang/tooling/.ext seeds (Spec §5.4, layer B).
//! Seeds are binary-embedded (`include_str!`) and copied into the project's
//! `contracts_dir` in one of three modes:
//!
//! * absent-only (`materialize_contracts(.., force=false)`) — the install default
//!   for a fresh target; never touches an existing file, so seeds age silently.
//! * `force` (`materialize_contracts(.., force=true)`) — the deliberate hammer:
//!   overwrites unconditionally, local changes included.
//! * lock-based (`refresh_contracts`) — uses `.lean-ctx/lean-md.lock` to tell a
//!   stale-but-untouched seed (heal it) from a user-edited one (`.new` beside it).
//!
//! Materializing a seed does not by itself decide what a render resolves to —
//! for the resolution order (built-in vs. project file) see `fragments.rs`. The
//! `*.ext.lmd.md` seeds EXTEND their built-in fragment: `FragmentRegistry::resolve`
//! appends the `.ext` after the built-in body, it never replaces it. They ship inert
//! (HTML comments only) so an untouched seed keeps every render byte-stable (#498).

use std::path::{Path, PathBuf};

/// Project-local macro library (`test`/`commit`/`tdd`) imported by every
/// generated `.lmd.md` plan. Module-level (not test-only) so Subplan-4-Task-2
/// can register it as a `PROJECT_SEEDS` entry without moving it.
const PLAN_RECIPES: &str = include_str!("../content/templates/plan-recipes.lmd.md");

/// Self-documenting `.lmd.md` plan skeleton (meta-head + one real `@phase`
/// example). Module-level for the same reason as `PLAN_RECIPES`.
const PLAN_TEMPLATE: &str = include_str!("../content/templates/plan-template.lmd.md");

/// (relative target path under contracts_dir, embedded content).
pub const PROJECT_SEEDS: &[(&str, &str)] = &[
    (
        "lang/rust.lmd.md",
        include_str!("../content/lang/rust.lmd.md"),
    ),
    (
        "tooling/mcp-tools.lmd.md",
        include_str!("../content/tooling/mcp-tools.lmd.md"),
    ),
    (
        "dispatch-contract.ext.lmd.md",
        include_str!("../content/templates/dispatch-contract.ext.lmd.md"),
    ),
    (
        "hard-rules.ext.lmd.md",
        include_str!("../content/templates/hard-rules.ext.lmd.md"),
    ),
    (
        "parallel-dispatch.ext.lmd.md",
        include_str!("../content/templates/parallel-dispatch.ext.lmd.md"),
    ),
    ("plan-recipes.lmd.md", PLAN_RECIPES),
    ("plan-template.lmd.md", PLAN_TEMPLATE),
];

/// Every sha256 each seed has ever shipped with (oldest first, current last),
/// checked in as `content/seeds.sha256` and parsed at runtime.
///
/// Why a checked-in manifest and not a third `PROJECT_SEEDS` field: the history
/// must outlive the bytes it describes. `include_str!` is build-invalidating —
/// edit a seed and no compiled code can see the OLD version any more. A manifest
/// is a snapshot the compiler does not drag along, which is exactly what makes
/// the append-only gate in `tests/seed_history.rs` able to bite (#498).
#[allow(dead_code)] // only `seed_history` (below) reads it until Task 3 wires the heal path.
const SEED_HISTORY_SRC: &str = include_str!("../content/seeds.sha256");

/// Parse `sha256sum`-format lines (`<hex>  <rel>`) into `rel → [hex]`, order
/// preserved. Comment and blank lines are skipped. Shared with the gate so both
/// sides read the manifest through one parser.
#[allow(dead_code)] // pub(crate): only `seed_history` (below) and the unit test call it until Task 3 wires it into the heal path.
pub(crate) fn parse_history(src: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some((hex, rel)) = t.split_once("  ") else {
            continue;
        };
        map.entry(rel.trim().to_string())
            .or_default()
            .push(hex.trim().to_string());
    }
    map
}

/// The history of one seed, by its `PROJECT_SEEDS` target key. Empty slice for an
/// unknown key — an unlisted seed simply has no history, never a panic.
#[allow(dead_code)] // wired into the refresh_contracts heal path in a later task (#498 Part A, Task 3); Task 1 only wires the manifest + parser and covers it from the unit test below.
fn seed_history(rel: &str) -> &'static [String] {
    static HISTORY: std::sync::OnceLock<std::collections::HashMap<String, Vec<String>>> =
        std::sync::OnceLock::new();
    HISTORY
        .get_or_init(|| parse_history(SEED_HISTORY_SRC))
        .get(rel)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Materialize embedded project seeds into `<project_root>/<contracts_dir>`.
/// `force=false` is absent-only (idempotent, never clobbers user edits); `force=true`
/// overwrites an existing target to refresh a stale derived seed after the embedded
/// copy changed. Returns the paths actually written.
pub fn materialize_contracts(
    project_root: &Path,
    contracts_dir: &str,
    force: bool,
) -> std::io::Result<Vec<PathBuf>> {
    let base = project_root.join(contracts_dir);
    let mut written = Vec::new();
    for (rel, content) in PROJECT_SEEDS {
        let target = base.join(rel);
        if !force && target.exists() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, content)?;
        written.push(target);
    }
    Ok(written)
}

/// What a `refresh_contracts` run did that the user may want to know about.
#[derive(Default)]
pub struct RefreshReport {
    /// Stale but untouched seeds that were silently updated to the embedded copy.
    pub healed: Vec<PathBuf>,
    /// Seeds carrying local changes (or unknown provenance) that the user has NOT
    /// acknowledged: left alone, embedded copy written beside them as `<target>.new`.
    /// This is the only conflict list worth a word — see `acked` for the rest.
    pub preserved: Vec<PathBuf>,
    /// Conflicts the user already acknowledged for exactly this embedded seed. Still
    /// preserved, still diverging — but there is nothing NEW to say, so they carry no
    /// `.new` file and no report line. Kept as a field because `ack` needs to see the
    /// standing conflicts, not because anyone should print them.
    pub acked: Vec<PathBuf>,
}

impl RefreshReport {
    /// Nothing happened that is worth a word to the user. An acknowledged conflict is
    /// deliberately quiet: the user answered this exact proposal already.
    pub fn is_quiet(&self) -> bool {
        self.healed.is_empty() && self.preserved.is_empty()
    }
}

/// `<target>.new` — the standing proposal beside a preserved seed.
fn new_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".new");
    target.with_file_name(name)
}

/// The conflict for `key` is gone (reverted or healed): drop the standing proposal and
/// the consent that answered it. Leaving either behind makes lean-md lie later — a
/// stale `.new` keeps `check` reporting "your local copies were kept" about a conflict
/// that no longer exists, and a spent ack would silence the user's NEXT edit.
/// Returns whether the lock changed.
fn resolve_conflict(
    lock: &mut crate::lock::Lock,
    key: &str,
    target: &Path,
) -> std::io::Result<bool> {
    let stale = new_path(target);
    if stale.exists() {
        std::fs::remove_file(&stale)?;
    }
    Ok(lock.clear_ack(key))
}

/// Lock key for a seed: paths in the lock are relative to `.lean-ctx/`, the
/// directory `sha256sum -c` runs in.
fn lock_key(contracts_dir: &str, rel: &str) -> String {
    let dir = contracts_dir
        .strip_prefix(".lean-ctx/")
        .unwrap_or(contracts_dir)
        .trim_end_matches('/');
    if dir.is_empty() {
        rel.to_string()
    } else {
        format!("{dir}/{rel}")
    }
}

/// Lock-based refresh — the third mode beside absent-only and `force`.
///
/// Absent-only lets seeds age; `force` clobbers real local work. The lock records
/// the seed hash this project was materialized with, which is what separates
/// "stale but untouched" (heal it) from "the user changed it" (never touch it,
/// drop the new copy beside it as `.new` and say so). A seed with no lock entry
/// has unknown provenance and is treated as user-owned.
///
/// A conflict the user acknowledged (`lean-md ack`) stays preserved but goes silent:
/// no `.new`, no report line. The lock's *provenance* entry is untouched either way,
/// so the seed can still heal the moment the user reverts.
pub fn refresh_contracts(
    project_root: &Path,
    contracts_dir: &str,
) -> std::io::Result<RefreshReport> {
    let base = project_root.join(contracts_dir);
    let mut lock = crate::lock::Lock::load(project_root);
    let mut report = RefreshReport::default();
    let mut lock_dirty = false;

    for (rel, content) in PROJECT_SEEDS {
        let target = base.join(rel);
        let key = lock_key(contracts_dir, rel);
        let embedded_hex = crate::hashx::sha256_hex(content.as_bytes());

        let Ok(local) = std::fs::read(&target) else {
            // Absent target: an install, not a user edit — write it and record it.
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, content)?;
            lock.set(&key, &embedded_hex);
            // A seed that was deleted outright takes its conflict with it.
            resolve_conflict(&mut lock, &key, &target)?;
            lock_dirty = true;
            continue;
        };
        let local_hex = crate::hashx::sha256_hex(&local);

        if local_hex == embedded_hex {
            // Already current — whatever conflict existed here is over (the user
            // reverted, or replaced their copy with the .new). Record provenance if the
            // lock did not know it yet, and clean up after the conflict.
            if lock.get(&key) != Some(embedded_hex.as_str()) {
                lock.set(&key, &embedded_hex);
                lock_dirty = true;
            }
            lock_dirty |= resolve_conflict(&mut lock, &key, &target)?;
            continue;
        }

        if lock.get(&key) == Some(local_hex.as_str()) {
            // Local matches what we last wrote → untouched, only stale → heal it. Any
            // `.new`/ack from an earlier, since-reverted conflict is spent too.
            std::fs::write(&target, content)?;
            lock.set(&key, &embedded_hex);
            resolve_conflict(&mut lock, &key, &target)?;
            lock_dirty = true;
            report.healed.push(target);
            continue;
        }

        // Local edit, or no lock entry (unknown provenance): never overwrite.
        //
        // Consent decides whether this is worth saying. `ack` records the EMBEDDED hash
        // the user answered; while it still matches, the divergence is old news and we
        // keep quiet — a report the user cannot switch off becomes wallpaper, and then
        // the real ones get scrolled past too. The moment the embedded seed moves on,
        // the ack no longer matches and the proposal is genuinely new again.
        //
        // Note what is NOT done here: the lock entry is never set to `local_hex`. That
        // would silence the report by declaring the user's edit our own provenance, and
        // this seed could then never heal again. Consent and provenance are two
        // questions; `Lock` answers them on two channels (see `lock.rs`).
        if lock.ack(&key) == Some(embedded_hex.as_str()) {
            // Acknowledged: the proposal was seen and declined. `ack` already removed the
            // `.new`; do not resurrect it.
            report.acked.push(target);
            continue;
        }

        // Unacknowledged → the user gets the current embedded copy beside their own,
        // but only when that actually says something new. Rewriting an identical `.new`
        // on every server start makes it look freshly changed each time. Written only
        // when missing (nothing there to read yet) or when it differs (the embedded seed
        // moved on, so the standing proposal is out of date).
        let new_target = new_path(&target);
        let up_to_date = std::fs::read(&new_target)
            .map(|existing| existing == content.as_bytes())
            .unwrap_or(false);
        if !up_to_date {
            std::fs::write(&new_target, content)?;
        }
        report.preserved.push(target);
    }

    if lock_dirty {
        lock.save(project_root)?;
    }
    Ok(report)
}

/// Which seeds `lean-md ack` would act on: preserved conflicts, acknowledged or not.
/// Pure read — `cmd_ack` uses it to resolve the user's filter arguments.
fn standing_conflicts(project_root: &Path, contracts_dir: &str) -> Vec<(String, PathBuf, String)> {
    let base = project_root.join(contracts_dir);
    let lock = crate::lock::Lock::load(project_root);
    let mut out = Vec::new();
    for (rel, content) in PROJECT_SEEDS {
        let target = base.join(rel);
        let embedded_hex = crate::hashx::sha256_hex(content.as_bytes());
        let Ok(local) = std::fs::read(&target) else {
            continue;
        };
        let local_hex = crate::hashx::sha256_hex(&local);
        if local_hex == embedded_hex {
            continue; // no conflict
        }
        let key = lock_key(contracts_dir, rel);
        if lock.get(&key) == Some(local_hex.as_str()) {
            continue; // untouched + stale → heals, never a conflict
        }
        out.push((key, target, embedded_hex));
    }
    out
}

/// Outcome of an `ack` run.
#[derive(Default)]
pub struct AckReport {
    /// Seeds whose conflict is now acknowledged (`.new` removed, consent recorded).
    pub acked: Vec<PathBuf>,
    /// Filter arguments that matched no standing conflict — the user asked about
    /// something that is not (or no longer) in conflict, and deserves to hear so
    /// rather than get a silent success.
    pub unmatched: Vec<String>,
}

/// Acknowledge seed conflicts: record the user's consent for the CURRENT embedded seed
/// and drop the `.new` beside it. `filter` empty → every standing conflict; otherwise a
/// path per entry, matched loosely (full path, path under the contracts dir, or bare
/// file name; a trailing `.new` is stripped, since that is the name `check` prints).
///
/// Writes (lock + `.new` removal), so it belongs to a verb the user invokes — never to
/// `render`/`check`, which stay purely reading (D-1).
pub fn ack_seeds(
    project_root: &Path,
    contracts_dir: &str,
    filter: &[String],
) -> std::io::Result<AckReport> {
    let conflicts = standing_conflicts(project_root, contracts_dir);
    let matches = |target: &Path, arg: &str| -> bool {
        let arg = arg.strip_suffix(".new").unwrap_or(arg);
        let arg = Path::new(arg);
        target == arg || target.ends_with(arg)
    };

    let mut lock = crate::lock::Lock::load(project_root);
    let mut report = AckReport::default();
    let mut dirty = false;
    for (key, target, embedded_hex) in &conflicts {
        if !filter.is_empty() && !filter.iter().any(|a| matches(target, a)) {
            continue;
        }
        lock.set_ack(key, embedded_hex);
        dirty = true;
        let proposal = new_path(target);
        if proposal.exists() {
            std::fs::remove_file(&proposal)?;
        }
        report.acked.push(target.clone());
    }
    for arg in filter {
        if !conflicts.iter().any(|(_, t, _)| matches(t, arg)) {
            report.unmatched.push(arg.clone());
        }
    }
    if dirty {
        lock.save(project_root)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Macro names defined in a plan-recipes source (`@define NAME(...)`).
    fn defined_macro_names(src: &str) -> std::collections::HashSet<String> {
        src.lines()
            .filter_map(|l| l.trim_start().strip_prefix("@define "))
            .filter_map(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
            .collect()
    }

    #[test]
    fn seeds_are_non_empty_and_unique() {
        assert!(!PROJECT_SEEDS.is_empty());
        let mut paths: Vec<&str> = PROJECT_SEEDS.iter().map(|(p, _)| *p).collect();
        let n = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), n, "duplicate seed target paths");
        for (_, content) in PROJECT_SEEDS {
            assert!(
                !content.trim().is_empty(),
                "embedded seed must be non-empty"
            );
        }
    }

    #[test]
    fn project_seeds_materialize() {
        let root = std::env::temp_dir().join(format!("lmd_pseeds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let written = materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        let base = root.join(".lean-ctx/lean-md");
        assert!(
            base.join("plan-recipes.lmd.md").exists(),
            "plan-recipes must materialize at root"
        );
        assert!(
            base.join("plan-template.lmd.md").exists(),
            "plan-template must materialize at root"
        );
        assert!(written.iter().any(|p| p.ends_with("plan-recipes.lmd.md")));

        // Absent-only: a second run writes nothing new.
        let again = materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        assert!(
            again.is_empty(),
            "second run must be idempotent (absent-only)"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn materialize_writes_then_is_idempotent() {
        let root = std::env::temp_dir().join(format!("lmd_seeds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";

        let first = materialize_contracts(&root, dir, false).unwrap();
        assert_eq!(
            first.len(),
            PROJECT_SEEDS.len(),
            "first run writes all seeds"
        );
        for (rel, _) in PROJECT_SEEDS {
            assert!(root.join(dir).join(rel).exists(), "seed not written: {rel}");
        }

        // Second run: targets exist → absent-only → writes nothing.
        let second = materialize_contracts(&root, dir, false).unwrap();
        assert!(
            second.is_empty(),
            "materialize must be idempotent (absent-only)"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn materialize_force_refreshes_stale_seed() {
        // M2: a stale local seed (an old derived copy) must be refreshed by force=true,
        // while force=false stays absent-only and leaves an existing target untouched.
        let root = std::env::temp_dir().join(format!("lmd_seeds_force_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";

        // Seed once, then overwrite a target with stale content.
        materialize_contracts(&root, dir, false).unwrap();
        let stale = root.join(dir).join("plan-recipes.lmd.md");
        std::fs::write(&stale, "# stale derived copy\n").unwrap();

        // Absent-only refuses to refresh it.
        let noop = materialize_contracts(&root, dir, false).unwrap();
        assert!(noop.is_empty(), "force=false must stay absent-only");
        assert_eq!(
            std::fs::read_to_string(&stale).unwrap(),
            "# stale derived copy\n",
            "force=false must not clobber an existing target"
        );

        // force=true rewrites it back to the embedded seed content.
        let refreshed = materialize_contracts(&root, dir, true).unwrap();
        assert!(
            refreshed.iter().any(|p| p.ends_with("plan-recipes.lmd.md")),
            "force=true must (re)write plan-recipes"
        );
        assert_eq!(
            std::fs::read_to_string(&stale).unwrap(),
            PLAN_RECIPES,
            "force=true must refresh the stale seed to the embedded content"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_heals_a_stale_untouched_seed_silently() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_heal_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        // First refresh writes the lock for the pristine tree.
        refresh_contracts(&root, dir).unwrap();

        // Simulate "the embedded seed moved on": pin an OLD hash in the lock and put the
        // matching old content on disk. Local == lock → untouched → may heal.
        let target = root.join(dir).join("plan-recipes.lmd.md");
        let old = "# an older embedded copy\n";
        std::fs::write(&target, old).unwrap();
        let mut lock = crate::lock::Lock::load(&root);
        lock.set(
            "lean-md/plan-recipes.lmd.md",
            &crate::hashx::sha256_hex(old.as_bytes()),
        );
        lock.save(&root).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            PLAN_RECIPES,
            "stale + untouched must heal to the embedded seed"
        );
        assert!(
            report
                .healed
                .iter()
                .any(|p| p.ends_with("plan-recipes.lmd.md"))
        );
        assert!(report.preserved.is_empty(), "no .new for an untouched seed");
        assert!(
            !target.with_extension("md.new").exists(),
            "must not litter a .new"
        );
        // The lock followed along, so the next run is a no-op.
        assert!(refresh_contracts(&root, dir).unwrap().is_quiet());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_never_overwrites_a_user_edit_and_writes_new_beside_it() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_edit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap(); // lock now matches the pristine tree

        let target = root.join(dir).join("lang/rust.lmd.md");
        let edit = "# my project rule\n";
        std::fs::write(&target, edit).unwrap(); // local != lock → user edit

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            edit,
            "a user edit must NEVER be clobbered"
        );
        let new_file = root.join(dir).join("lang/rust.lmd.md.new");
        assert!(
            new_file.exists(),
            ".new must be written beside the edited seed"
        );
        assert!(report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")));
        assert!(report.healed.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Drive a tree into the preserve case: seeds materialized, lock written for the
    /// pristine tree, then one seed locally edited. Returns (root, edited target).
    fn tree_with_a_user_edited_seed(tag: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("lmd_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();
        let target = root.join(dir).join("lang/rust.lmd.md");
        std::fs::write(&target, "# my project rule\n").unwrap();
        (root, target)
    }

    fn new_path_of(target: &Path) -> PathBuf {
        let mut name = target.file_name().unwrap().to_os_string();
        name.push(".new");
        target.with_file_name(name)
    }

    #[test]
    fn an_unchanged_new_file_is_not_rewritten() {
        // Rewriting an identical .new on every server start is what turns a real signal
        // into wallpaper: the file's mtime keeps saying "brand new" when nothing changed.
        // Proof: pin an old mtime, refresh, and require the mtime to survive.
        let (root, target) = tree_with_a_user_edited_seed("new_stable");
        let dir = ".lean-ctx/lean-md";
        refresh_contracts(&root, dir).unwrap();
        let new_file = new_path_of(&target);
        assert!(new_file.exists(), "preserve must drop a .new first");

        let pinned = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::File::options()
            .write(true)
            .open(&new_file)
            .unwrap()
            .set_modified(pinned)
            .unwrap();

        refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::metadata(&new_file).unwrap().modified().unwrap(),
            pinned,
            "an identical .new must not be touched again"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_stale_new_file_is_refreshed_to_the_current_seed() {
        // The user must see the CURRENT proposal, not one from three binaries ago.
        let (root, target) = tree_with_a_user_edited_seed("new_stale");
        let dir = ".lean-ctx/lean-md";
        let new_file = new_path_of(&target);
        std::fs::write(&new_file, "# a proposal from an older binary\n").unwrap();

        refresh_contracts(&root, dir).unwrap();
        let embedded = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "lang/rust.lmd.md")
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&new_file).unwrap(),
            embedded,
            "a stale .new must be refreshed to the embedded seed"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_preserved_user_edit_never_becomes_the_new_provenance() {
        // Option (b) — adopting the local hash as provenance — was explicitly rejected:
        // it silences the report forever but also disables healing for good. The lock
        // entry must keep pointing at what WE last wrote.
        let (root, target) = tree_with_a_user_edited_seed("new_prov");
        let dir = ".lean-ctx/lean-md";
        let before = crate::lock::Lock::load(&root)
            .get("lean-md/lang/rust.lmd.md")
            .map(str::to_string);
        let local_hex = crate::hashx::sha256_hex(&std::fs::read(&target).unwrap());

        refresh_contracts(&root, dir).unwrap();

        let after = crate::lock::Lock::load(&root)
            .get("lean-md/lang/rust.lmd.md")
            .map(str::to_string);
        assert_ne!(
            after.as_deref(),
            Some(local_hex.as_str()),
            "preserve must not adopt the user edit as provenance"
        );
        assert_eq!(after, before, "preserve must leave the lock entry alone");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_deleted_new_file_comes_back_because_the_divergence_did_not_go_away() {
        // Deleting the .new is not a resolution, it is a dismissal: the seed still
        // diverges from the binary, so the proposal is still standing. Saying "I keep
        // mine" is what `ack` is for — and it is the ONLY thing that silences this.
        let (root, target) = tree_with_a_user_edited_seed("new_revive");
        let dir = ".lean-ctx/lean-md";
        refresh_contracts(&root, dir).unwrap();
        let new_file = new_path_of(&target);
        std::fs::remove_file(&new_file).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            new_file.exists(),
            "the divergence persists, so the proposal must reappear"
        );
        assert!(report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_acked_conflict_goes_quiet() {
        // The whole point: the user must be able to say "I keep mine" and be believed.
        // A report that cannot be switched off becomes wallpaper, and then the real ones
        // get scrolled past too.
        let (root, target) = tree_with_a_user_edited_seed("ack_quiet");
        let dir = ".lean-ctx/lean-md";
        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")),
            "precondition: the conflict must speak before it is acked"
        );
        assert!(new_path_of(&target).exists());

        let ack = ack_seeds(&root, dir, &[]).unwrap();
        assert!(ack.acked.iter().any(|p| p.ends_with("rust.lmd.md")));
        assert!(ack.unmatched.is_empty());
        assert!(
            !new_path_of(&target).exists(),
            "ack must clear the standing proposal — the user has seen it"
        );

        // And it stays quiet across any number of further runs.
        for _ in 0..3 {
            let report = refresh_contracts(&root, dir).unwrap();
            assert!(report.is_quiet(), "an acked conflict must not speak again");
            assert!(report.preserved.is_empty());
            assert!(
                report.acked.iter().any(|p| p.ends_with("rust.lmd.md")),
                "still preserved, just silent"
            );
            assert!(
                !new_path_of(&target).exists(),
                "an acked conflict must not resurrect its .new"
            );
        }
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# my project rule\n",
            "ack must never touch the user's copy"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_acked_conflict_speaks_again_when_the_seed_moves_on() {
        // Consent is for THIS proposal, not for every future one. Once the embedded seed
        // moves on there is something genuinely new to say, so the report must return.
        let (root, target) = tree_with_a_user_edited_seed("ack_moves_on");
        let dir = ".lean-ctx/lean-md";
        refresh_contracts(&root, dir).unwrap();
        ack_seeds(&root, dir, &[]).unwrap();
        assert!(refresh_contracts(&root, dir).unwrap().is_quiet());

        // Simulate a newer binary: the ack now names a hash the embedded seed no longer
        // has. (The lock's ack channel is the only moving part — the seed content is
        // compiled in, so we rewrite the recorded consent to an older hash instead.)
        let key = "lean-md/lang/rust.lmd.md";
        let mut lock = crate::lock::Lock::load(&root);
        lock.set_ack(
            key,
            &crate::hashx::sha256_hex(b"a seed from an older binary\n"),
        );
        lock.save(&root).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")),
            "a moved-on seed is a NEW proposal and must speak again"
        );
        assert!(!report.is_quiet());
        assert!(
            new_path_of(&target).exists(),
            "the new proposal must be laid down for the user to read"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn acking_does_not_become_the_new_provenance() {
        // Option (b) stays rejected: the lock must NOT take the local hash, or the seed
        // could never heal again. Ack is consent, the lock is provenance.
        let (root, target) = tree_with_a_user_edited_seed("ack_prov");
        let dir = ".lean-ctx/lean-md";
        let key = "lean-md/lang/rust.lmd.md";
        let before = crate::lock::Lock::load(&root).get(key).map(str::to_string);
        let local_hex = crate::hashx::sha256_hex(&std::fs::read(&target).unwrap());

        refresh_contracts(&root, dir).unwrap();
        ack_seeds(&root, dir, &[]).unwrap();

        let lock = crate::lock::Lock::load(&root);
        assert_ne!(
            lock.get(key),
            Some(local_hex.as_str()),
            "ack must not adopt the user's edit as provenance"
        );
        assert_eq!(
            lock.get(key).map(str::to_string),
            before,
            "ack must leave the provenance entry exactly as it was"
        );

        // Proof that healing still works: revert to what we last wrote, and the seed
        // heals to the embedded copy — which option (b) would have made impossible.
        let provenance = before.expect("the pristine tree must have a lock entry");
        let embedded = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "lang/rust.lmd.md")
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(
            provenance,
            crate::hashx::sha256_hex(embedded.as_bytes()),
            "provenance still names the embedded seed"
        );
        std::fs::write(&target, embedded).unwrap();
        let report = refresh_contracts(&root, dir).unwrap();
        assert!(report.is_quiet());
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            embedded,
            "the seed can still be healed/reverted after an ack"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_reverted_edit_cleans_up_its_orphaned_new_file() {
        // Reporting "your local copies were kept" about a conflict that no longer exists
        // is a false statement — worse than silence.
        let (root, target) = tree_with_a_user_edited_seed("ack_revert");
        let dir = ".lean-ctx/lean-md";
        refresh_contracts(&root, dir).unwrap();
        let new_file = new_path_of(&target);
        assert!(new_file.exists(), "precondition: a proposal is standing");

        // The user reverts to the embedded seed — the conflict is settled.
        let embedded = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "lang/rust.lmd.md")
            .map(|(_, c)| *c)
            .unwrap();
        std::fs::write(&target, embedded).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            !new_file.exists(),
            "the orphaned .new must go — its conflict is over"
        );
        assert!(report.is_quiet());
        assert!(report.preserved.is_empty() && report.acked.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_spent_ack_does_not_silence_the_next_edit() {
        // An ack answers one conflict. Once that conflict is settled the consent is
        // spent — leaving it behind would silence a later, unrelated edit that the user
        // has never seen a proposal for.
        let (root, target) = tree_with_a_user_edited_seed("ack_spent");
        let dir = ".lean-ctx/lean-md";
        refresh_contracts(&root, dir).unwrap();
        ack_seeds(&root, dir, &[]).unwrap();
        assert!(refresh_contracts(&root, dir).unwrap().is_quiet());

        // Revert (conflict settled) …
        let embedded = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "lang/rust.lmd.md")
            .map(|(_, c)| *c)
            .unwrap();
        std::fs::write(&target, embedded).unwrap();
        refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            crate::lock::Lock::load(&root).ack("lean-md/lang/rust.lmd.md"),
            None,
            "a settled conflict must not leave consent behind"
        );

        // … then edit again. This is a fresh conflict and must be reported.
        std::fs::write(&target, "# a different rule\n").unwrap();
        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")),
            "a new edit must speak — the old ack was spent"
        );
        assert!(new_path_of(&target).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ack_takes_a_path_filter_and_names_what_it_did_not_match() {
        // The user acks one seed by the name `check` printed (with or without `.new`);
        // the others keep speaking. An argument that matches nothing must say so rather
        // than report a silent success.
        let (root, target) = tree_with_a_user_edited_seed("ack_filter");
        let dir = ".lean-ctx/lean-md";
        let other = root.join(dir).join("tooling/mcp-tools.lmd.md");
        std::fs::write(&other, "# my tool notes\n").unwrap();
        refresh_contracts(&root, dir).unwrap();

        let ack = ack_seeds(&root, dir, &["lang/rust.lmd.md.new".to_string()]).unwrap();
        assert_eq!(ack.acked.len(), 1, "only the named seed is acked");
        assert!(ack.acked[0].ends_with("rust.lmd.md"));
        assert!(ack.unmatched.is_empty());
        assert!(!new_path_of(&target).exists());
        assert!(
            new_path_of(&other).exists(),
            "an un-named conflict keeps its proposal"
        );

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            report
                .preserved
                .iter()
                .any(|p| p.ends_with("mcp-tools.lmd.md")),
            "the un-acked conflict must still speak"
        );
        assert!(!report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")));

        let miss = ack_seeds(&root, dir, &["lang/nope.lmd.md".to_string()]).unwrap();
        assert!(miss.acked.is_empty());
        assert_eq!(miss.unmatched, vec!["lang/nope.lmd.md".to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn legacy_tree_without_a_lock_is_treated_conservatively() {
        // Today's state: 4 stale seeds, no lock, provenance unknown. We must not guess
        // "untouched" — that would clobber whatever the user did before locks existed.
        let root = std::env::temp_dir().join(format!("lmd_refresh_legacy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        let target = root.join(dir).join("tooling/mcp-tools.lmd.md");
        std::fs::write(&target, "# a pre-lock local copy\n").unwrap();
        assert!(!root.join(".lean-ctx/lean-md.lock").exists());

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# a pre-lock local copy\n",
            "unknown provenance must never be overwritten"
        );
        assert!(root.join(dir).join("tooling/mcp-tools.lmd.md.new").exists());
        assert!(
            report
                .preserved
                .iter()
                .any(|p| p.ends_with("mcp-tools.lmd.md"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_of_a_current_tree_is_a_silent_noop() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_noop_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            report.is_quiet(),
            "a current tree must produce no report at all"
        );
        assert!(report.healed.is_empty() && report.preserved.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_newly_registered_seed_materializes_without_a_new_file() {
        // task-5 adds two seeds AFTER locks exist in the field. An absent target is not a
        // user edit — it must just appear, silently.
        let root = std::env::temp_dir().join(format!("lmd_refresh_fresh_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();

        let (rel, _) = PROJECT_SEEDS[0];
        std::fs::remove_file(root.join(dir).join(rel)).unwrap();
        let report = refresh_contracts(&root, dir).unwrap();
        assert!(
            root.join(dir).join(rel).exists(),
            "absent seed must be (re)written"
        );
        assert!(
            report.preserved.is_empty(),
            "an absent target is not a user edit"
        );
        assert!(!root.join(dir).join(format!("{rel}.new")).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_recipes_import() {
        // @import plan-recipes + @call test(...) expands, and vars.toml overrides the
        // inline @var default (test_cmd). jail_root = a materialized seed tree.
        let root = std::env::temp_dir().join(format!("lmd_recipes_{}", std::process::id()));
        let vars_dir = root.join(".lean-ctx/lean-md");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&vars_dir).unwrap();
        materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        // PLAN_RECIPES is not yet a PROJECT_SEEDS entry (that wiring lands in
        // Subplan-4-Task-2), so stage it directly at the resolver's target path
        // for this test — same target Task-2's PROJECT_SEEDS entry will use.
        std::fs::write(vars_dir.join("plan-recipes.lmd.md"), PLAN_RECIPES).unwrap();
        std::fs::write(
            vars_dir.join("vars.toml"),
            "test_cmd = \"cargo nextest run\"\n",
        )
        .unwrap();

        let src = "\
@lean-md
consumer: ai

@var test_cmd default=\"cargo test\"
@import .lean-ctx/lean-md/plan-recipes /
@phase \"task-1\"
@call test(demo)
@phase-end
";
        let out =
            crate::skills::render_source_with_phase(src, Some("task-1"), None, None, root.clone())
                .unwrap();
        assert!(
            out.contains("cargo nextest run demo"),
            "recipe did not expand with vars override: {out}"
        );
        assert!(!out.contains("@call test"), "@call not expanded: {out}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_template_header_declares_crp_compact() {
        // Terseness rework: the template header binds crp: compact (drives the dispatch
        // CRP line + apply_crp_hook deterministically), alongside consumer: ai.
        assert!(
            PLAN_TEMPLATE.contains("consumer: ai"),
            "template header must keep consumer: ai"
        );
        assert!(
            PLAN_TEMPLATE.contains("crp: compact"),
            "template header must declare crp: compact"
        );
    }

    #[test]
    fn plan_template_meta_declares_lint_cmd() {
        // The meta-head declares lint_cmd once (pattern of test_cmd); vars.toml wins.
        assert!(
            PLAN_TEMPLATE.contains("@var lint_cmd"),
            "template meta-head must declare @var lint_cmd"
        );
    }

    #[test]
    fn plan_template_self_documents() {
        // Self-documenting: guidance markers present, no superpowers token.
        assert!(PLAN_TEMPLATE.contains("One @phase per task"));
        assert!(PLAN_TEMPLATE.contains("@call test"));
        assert!(PLAN_TEMPLATE.contains("anchor it"));
        assert!(!PLAN_TEMPLATE.to_lowercase().contains("superpowers"));

        // The real example task renders cleanly against a materialized seed tree.
        let root = std::env::temp_dir().join(format!("lmd_tmpl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        materialize_contracts(&root, ".lean-ctx/lean-md", false).unwrap();
        // plan-template's meta-head imports plan-recipes; same staging note as
        // plan_recipes_import above (PROJECT_SEEDS wiring lands in Task-2).
        std::fs::write(
            root.join(".lean-ctx/lean-md/plan-recipes.lmd.md"),
            PLAN_RECIPES,
        )
        .unwrap();

        let out = crate::skills::render_source_with_phase(
            PLAN_TEMPLATE,
            Some("task-1"),
            None,
            None,
            root.clone(),
        )
        .unwrap();
        assert!(
            out.contains("foo_adds_one"),
            "example task did not render the test recipe: {out}"
        );
        assert!(
            out.contains("pub fn foo"),
            "new-code block missing from rendered task: {out}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_recipes_all_documented() {
        // Every @define's first non-empty body line is an HTML-comment description,
        // so the --signatures index (Subplan 1) carries a doc line for each macro.
        let lines: Vec<&str> = PLAN_RECIPES.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("@define ") {
                let doc = lines[i + 1..]
                    .iter()
                    .find(|l| !l.trim().is_empty())
                    .copied()
                    .unwrap_or("");
                assert!(
                    doc.trim_start().starts_with("<!--"),
                    "@define on line {} lacks a description comment: {line}",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn plan_recipes_carry_code_intel_macros() {
        // §4/§4a: the macro library must expose the code-intel recipes so plans can
        // @call them — presence is the enforced contract, not a suggestion.
        let defined = defined_macro_names(PLAN_RECIPES);
        for name in [
            "verify",
            "review_change",
            "check_smells",
            "inspect",
            "reformat_commit",
            "remember_decision",
            "recall_context",
            "callers",
        ] {
            assert!(
                defined.contains(name),
                "plan-recipes must define the {name} code-intel macro"
            );
        }
    }

    #[test]
    fn plan_recipes_carry_gate_and_render_check() {
        // Terseness rework: the recipe layer must expose the pre-commit gate and the
        // lmd render smoke so plans can @call them.
        let defined = defined_macro_names(PLAN_RECIPES);
        assert!(
            defined.contains("gate"),
            "plan-recipes must define the gate quality-bar recipe"
        );
        assert!(
            defined.contains("render_check"),
            "plan-recipes must define the render_check smoke recipe"
        );
    }

    #[test]
    fn no_orphan_call() {
        // Every @call NAME(...) starting a line in plan-template hits a @define NAME(...)
        // in plan-recipes (static check; runtime already surfaces `macro not found`).
        let defined = defined_macro_names(PLAN_RECIPES);
        assert!(defined.contains("test") && defined.contains("commit") && defined.contains("tdd"));

        for line in PLAN_TEMPLATE.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("@call ") {
                let name = rest.split('(').next().unwrap_or("").trim().to_string();
                assert!(
                    defined.contains(&name),
                    "@call {name} in plan-template has no matching @define in plan-recipes"
                );
            }
        }
    }

    #[test]
    fn plan_template_has_verify_and_close_contract() {
        // §6: every task ends with the fixed Verify & Close sequence; conditional
        // slots hang on observable predicates (refactor / multi-file / prior-task).
        assert!(
            PLAN_TEMPLATE.contains("Verify & Close"),
            "template must define the Verify & Close sequence"
        );
        for call in [
            "@call verify(",
            "@call gate(",
            "@call commit(",
            "@call remember_decision(",
        ] {
            assert!(
                PLAN_TEMPLATE.contains(call),
                "Verify & Close must include {call}"
            );
        }
        assert!(
            PLAN_TEMPLATE.contains("@call recall_context(")
                && PLAN_TEMPLATE.contains("@call callers(")
                && PLAN_TEMPLATE.contains("@call review_change("),
            "template must offer the conditional slots (recall/callers/review_change)"
        );
    }

    #[test]
    fn mcp_tools_is_a_usage_reference() {
        // §5a: tooling/mcp-tools is the directive USAGE reference for plan authors —
        // one line per woven directive: purpose · minimal form · when-to-use.
        let seed = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "tooling/mcp-tools.lmd.md")
            .map(|(_, c)| *c)
            .expect("mcp-tools seed must be registered");
        for directive in [
            "@refactor",
            "@review",
            "@smells",
            "@graph",
            "@impact",
            "@recall",
            "@remember",
        ] {
            assert!(
                seed.contains(directive),
                "mcp-tools usage reference must document {directive}"
            );
        }
        assert!(
            seed.contains("Use") || seed.contains("Nutze"),
            "usage reference must carry when-to-use guidance"
        );
    }

    #[test]
    fn rust_lang_pack_pins_refactor_rule() {
        // §7: Rust tasks with rename/move/extract must instruct @refactor
        // (ctx_refactor) — no hand-edits; @edit only for non-symbol changes.
        let seed = PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "lang/rust.lmd.md")
            .map(|(_, c)| *c)
            .expect("rust lang seed must be registered");
        assert!(
            seed.contains("@refactor"),
            "rust pack must instruct @refactor"
        );
        assert!(
            seed.contains("rename") && seed.contains("extract"),
            "rust pack must name the refactor ops"
        );
        assert!(
            seed.contains("@edit"),
            "rust pack must scope @edit to non-symbol changes"
        );
    }

    #[test]
    fn history_parses_and_lists_hashes_oldest_first() {
        let h = seed_history("lang/rust.lmd.md");
        assert!(h.len() >= 2, "rust seed has a real history: {}", h.len());
        // Current embedded copy is the LAST entry — the invariant the heal path relies on.
        let embedded = crate::hashx::sha256_hex(
            PROJECT_SEEDS
                .iter()
                .find(|(p, _)| *p == "lang/rust.lmd.md")
                .unwrap()
                .1
                .as_bytes(),
        );
        assert_eq!(h.last().unwrap(), &embedded, "current hash is last");
        // A hash the project shipped earlier is still in the list.
        assert!(
            h.iter().any(|x| x.starts_with("48dd2f30a4461244")),
            "the stale-in-the-wild rust hash is remembered"
        );
        assert!(
            seed_history("nope.lmd.md").is_empty(),
            "unknown key → empty"
        );
        // Comment lines never leak in as keys.
        assert!(!parse_history(SEED_HISTORY_SRC).contains_key("#"));
    }
}
