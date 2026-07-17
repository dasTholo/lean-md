//! Pack version gate — checks the installed skills pack against the range this
//! binary expects.
//!
//! Binary and pack carry independent SemVer (#727 cut). Divergence *is* the point:
//! a content-only fix moves the pack while the binary stays put. So the gate warns
//! only on a RANGE violation, never on mere inequality — a check that flags the
//! intended normal case is noise from day one.

use std::path::Path;

/// The pack range this binary expects. Mirrors `[[dependencies]] version_req` in
/// lean-ctx-addon.toml — kept honest by `const_matches_the_addon_manifest`.
pub const PACK_VERSION_REQ: &str = "^0.2";

/// The pack this binary depends on. Matched case-insensitively against the lock's
/// package `name` — kept honest by `the_pack_name_matches_the_addon_manifest`.
const PACK_NAME: &str = "@dastholo/lean-md-skills";

/// Installed pack version from `.lean-ctx/ctxpkg.lock`. READ-ONLY — that file belongs
/// to lean-ctx (`pack install` generates it). Absent/unparsable → None, never an error.
pub fn installed_pack_version(project_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(project_root.join(".lean-ctx/ctxpkg.lock")).ok()?;
    // Keys are tracked per block rather than assuming `name` precedes `version`: TOML
    // tables are unordered by spec, so arrival order is lean-ctx's serialisation choice,
    // not a contract. Deciding as soon as both keys of one block are known keeps a real
    // violation from going quiet just because the file was written the other way round.
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            // Any table header opens a fresh block — nothing carries across.
            name = None;
            version = None;
            continue;
        }
        let Some((key, value)) = t.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        match key.trim() {
            // Registry namespaces are case-insensitive by intent (the registry
            // canonicalises to lowercase), and this project has a live casing split:
            // the ctxpkg token is scoped to `dastholo` while the codebase authors
            // `@dasTholo`. An exact compare would let a CamelCase lock silence a real
            // range violation — the gate would go quiet exactly when it must speak.
            "name" => name = Some(value.to_string()),
            "version" => version = Some(value.to_string()),
            _ => continue,
        }
        if let (Some(n), Some(v)) = (&name, &version)
            && n.eq_ignore_ascii_case(PACK_NAME)
        {
            return Some(v.clone());
        }
    }
    None
}

/// `MAJOR.MINOR.PATCH` → (major, minor, patch); a missing component reads as 0, so
/// `0.2` parses as `0.2.0`. Pre-release/build suffixes are cut at the first `-`/`+`.
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.split(['-', '+']).next()?;
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let part = |p: Option<&str>| -> Option<u64> { p.map_or(Some(0), |s| s.parse().ok()) };
    let minor = part(it.next())?;
    let patch = part(it.next())?;
    Some((major, minor, patch))
}

/// Caret-range check, done by hand (no `semver` dep). `^X.Y.Z` means: at least
/// `X.Y.Z`, and below the next incompatible release — the next MAJOR for `X > 0`,
/// but the next MINOR inside `0.x`, where minor bumps may break.
fn satisfies(req: &str, version: &str) -> bool {
    let Some(lower) = req.strip_prefix('^').and_then(parse_version) else {
        // An unknown range shape must not fabricate a violation.
        return true;
    };
    let Some(v) = parse_version(version) else {
        return true;
    };
    if v < lower {
        return false;
    }
    if lower.0 == 0 {
        v.0 == 0 && v.1 == lower.1
    } else {
        v.0 == lower.0
    }
}

/// Some(warning) only when the installed version is OUTSIDE the range.
pub fn drift_warning(project_root: &Path) -> Option<String> {
    let installed = installed_pack_version(project_root)?;
    if satisfies(PACK_VERSION_REQ, &installed) {
        return None;
    }
    Some(format!(
        "lean-md: installed pack {PACK_NAME} {installed} is outside the range this binary \
         expects ({PACK_VERSION_REQ}) — rendering may fail or silently use the wrong content"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_outside_the_range_warns() {
        let root = std::env::temp_dir().join(format!("lmd_vg_out_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        let w = drift_warning(&root).expect("0.3.0 is outside ^0.2 → must warn");
        assert!(w.contains("0.3.0") && w.contains("^0.2"), "{w}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_pack_inside_the_range_is_silent_even_when_it_differs_from_the_binary() {
        // The #727 cut's whole point: a content-only fix moves the pack to 0.2.1 while the
        // binary stays 0.2.0. Warning here would be noise from day one.
        let root = std::env::temp_dir().join(format!("lmd_vg_in_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.2.7\"\n",
        )
        .unwrap();
        assert_eq!(
            drift_warning(&root),
            None,
            "inequality inside the range is intended"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_absent_lock_is_neither_an_error_nor_output() {
        let root = std::env::temp_dir().join(format!("lmd_vg_absent_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(installed_pack_version(&root), None);
        assert_eq!(drift_warning(&root), None);
    }

    #[test]
    fn ctxpkg_lock_is_never_written() {
        // That file belongs to lean-ctx. Read-only, byte for byte.
        let root = std::env::temp_dir().join(format!("lmd_vg_ro_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        let raw = "[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.2.0\"\n";
        let path = root.join(".lean-ctx/ctxpkg.lock");
        std::fs::write(&path, raw).unwrap();
        let _ = drift_warning(&root);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn const_matches_the_addon_manifest() {
        // Same shape as the fragment-consistency gate: divergence falls in CI, not on a user.
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lean-ctx-addon.toml"),
        )
        .unwrap();
        assert!(
            manifest.contains(&format!("version_req = \"{PACK_VERSION_REQ}\"")),
            "PACK_VERSION_REQ drifted from lean-ctx-addon.toml"
        );
    }

    #[test]
    fn a_camelcase_pack_name_still_finds_the_version() {
        // The project has a live casing conflict: the ctxpkg token is scoped to
        // `dastholo` while the codebase authors `@dasTholo`. A case-sensitive compare
        // turns a real range violation into silence — the defect class this package
        // removes, rebuilt inside the check meant to remove it.
        let root = std::env::temp_dir().join(format!("lmd_vg_case_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nname = \"@dasTholo/lean-md-skills\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        assert_eq!(
            installed_pack_version(&root).as_deref(),
            Some("0.3.0"),
            "a CamelCase namespace must not hide the version"
        );
        assert!(
            drift_warning(&root).is_some(),
            "0.3.0 is outside ^0.2 — casing must not silence a real violation"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_pack_name_matches_the_addon_manifest() {
        // Without this, a rename in the manifest silently retires the whole gate:
        // installed_pack_version would never match again and drift_warning would be
        // None forever. Same shape as the version_req gate — divergence falls in CI.
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lean-ctx-addon.toml"),
        )
        .unwrap();
        assert!(
            manifest.to_ascii_lowercase().contains(&format!(
                "name        = \"{}\"",
                PACK_NAME.to_ascii_lowercase()
            )),
            "PACK_NAME drifted from the [[dependencies]] block in lean-ctx-addon.toml"
        );
    }

    #[test]
    fn key_order_inside_the_package_block_does_not_matter() {
        // TOML tables are unordered by spec, so `version` before `name` is a legal lock.
        // Relying on arrival order would make a real range violation depend on how
        // lean-ctx happens to serialise the file — silence by luck.
        let root = std::env::temp_dir().join(format!("lmd_vg_order_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nversion = \"0.3.0\"\nname = \"@dastholo/lean-md-skills\"\n",
        )
        .unwrap();
        assert_eq!(installed_pack_version(&root).as_deref(), Some("0.3.0"));
        assert!(drift_warning(&root).is_some(), "0.3.0 is outside ^0.2");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_version_from_a_foreign_package_block_is_never_borrowed() {
        // The risk of tracking keys per block: leaking another package's version into
        // ours. A block boundary must drop everything the previous block established.
        let root = std::env::temp_dir().join(format!("lmd_vg_foreign_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nname = \"@someone/other\"\nversion = \"9.9.9\"\n\
             \n[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        assert_eq!(installed_pack_version(&root).as_deref(), Some("0.2.0"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
