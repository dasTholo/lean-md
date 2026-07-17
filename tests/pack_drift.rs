//! Drift gate (#727/#498): `content/skills` must hash to the checked-in manifest.
//!
//! Published pack versions are immutable — the lockfile pins `artifact_sha256`.
//! Any content change therefore forces a PACK version bump plus a republish, never
//! a binary bump. This gate is what makes "content changed, bump forgotten" loud.
//!
//! Bless a legitimate change with:
//!     LEAN_MD_BLESS=1 cargo nextest run --test pack_drift
//! then bump the pack version and republish (see docs/dev-readme.md).
//!
//! The hash is lean-md's own definition, independent of lean-ctx's `content_hash`
//! (which compresses before hashing). The CI job `pack-drift.yml` cross-checks the
//! two against the real lean-ctx binary.

use std::path::{Path, PathBuf};

fn skills_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("content/skills")
}

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("content/skills.sha256")
}

/// Relative `/`-separated paths of every regular file under `root`, sorted by byte
/// order — the same collection rule lean-ctx's `collect_files` applies (dotfiles,
/// `node_modules`, `target` and symlinks skipped).
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        let ft = entry.file_type().expect("file type");
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            collect(root, &entry.path(), out);
        } else if ft.is_file() {
            let rel = entry
                .path()
                .strip_prefix(root)
                .expect("under root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push(rel);
        }
    }
}

fn render_manifest() -> String {
    let root = skills_root();
    let mut rels = Vec::new();
    collect(&root, &root, &mut rels);
    rels.sort();
    assert!(!rels.is_empty(), "content/skills is empty");
    let mut out = String::from(
        "# lean-md skills-pack content manifest (#498, #727)\n\
         # Regenerate: LEAN_MD_BLESS=1 cargo nextest run --test pack_drift\n\
         # A changed hash means: bump the PACK version and republish it.\n\
         # The binary version is untouched — the two SemVer lines are independent.\n",
    );
    for rel in &rels {
        let bytes = std::fs::read(root.join(rel)).expect("read file");
        let hex = lean_md::hashx::sha256_hex(&bytes);
        out.push_str(&format!("{hex}  {rel}\n"));
    }
    out
}

#[test]
fn skills_content_matches_the_checked_in_manifest() {
    let rendered = render_manifest();
    let path = manifest_path();
    if std::env::var("LEAN_MD_BLESS").is_ok() {
        std::fs::write(&path, &rendered).expect("write manifest");
        return;
    }
    let checked_in = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        checked_in, rendered,
        "content/skills drifted from content/skills.sha256.\n\
         Bless with: LEAN_MD_BLESS=1 cargo nextest run --test pack_drift\n\
         then bump the pack version and republish (docs/dev-readme.md)."
    );
}

#[test]
fn every_manifest_entry_names_a_file_that_exists() {
    let root = skills_root();
    let manifest = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let mut seen = 0usize;
    for line in manifest.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (_, rel) = line.split_once("  ").expect("`<sha256>  <relpath>`");
        assert!(
            root.join(rel).is_file(),
            "manifest names a missing file: {rel}"
        );
        seen += 1;
    }
    assert!(seen >= 30, "suspiciously few entries: {seen}");
}

#[test]
fn manifest_hash_uses_the_library_single_source() {
    // The gate and the runtime lock must never disagree on "how we hash".
    // `Digest` must be in scope even for the `Sha256::new()` UFCS call.
    use sha2::Digest;
    let bytes = b"lean-md drift probe";
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    let local: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(local, lean_md::hashx::sha256_hex(bytes));
}
