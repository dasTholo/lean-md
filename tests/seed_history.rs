//! Seed-history gate (#498): every embedded seed's CURRENT hash must be listed in
//! `content/seeds.sha256`, and no line ever leaves that file.
//!
//! Bless a legitimate seed change with:
//!     LEAN_MD_BLESS=1 cargo nextest run --test seed_history
//!
//! The bless APPENDS. It must never replace, and this is not a style preference:
//! `refresh_contracts` heals an installation without a lock by matching the local
//! bytes against this history. Drop a line and every project still carrying that
//! version silently stops healing forever.
//!
//! What this gate does NOT test: that the manifest's last hash equals the file on
//! disk *right now* by re-reading it — `include_str!` is build-invalidating, so
//! that comparison moves with the compiler and is a tautology. It tests the
//! embedded const against a checked-in snapshot, which does not move.

use std::path::{Path, PathBuf};

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("content/seeds.sha256")
}

/// `(hex, rel)` per manifest data line, order preserved. Deliberately NOT
/// `seeds::parse_history`: "did a line go missing" is a line-level question, and a
/// map answers it wrong (no order, duplicates collapsed).
fn manifest_lines(src: &str) -> Vec<(String, String)> {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once("  "))
        .map(|(h, r)| (h.trim().to_string(), r.trim().to_string()))
        .collect()
}

#[test]
fn every_seeds_current_hash_is_in_the_history() {
    let src = std::fs::read_to_string(manifest_path()).expect("content/seeds.sha256 readable");
    let lines = manifest_lines(&src);
    let mut missing: Vec<String> = Vec::new();
    for (rel, content) in lean_md::seeds::PROJECT_SEEDS {
        let hex = lean_md::hashx::sha256_hex(content.as_bytes());
        let known = lines.iter().any(|(h, r)| h == &hex && r == rel);
        if !known {
            missing.push(format!("{hex}  {rel}"));
        }
    }
    if missing.is_empty() {
        return;
    }
    if std::env::var("LEAN_MD_BLESS").is_ok() {
        std::fs::write(manifest_path(), append_missing(&src, &missing)).expect("append history");
        return;
    }
    panic!(
        "Seed changed, but its new hash is not in content/seeds.sha256:\n  {}\n\
         Bless (append-only): LEAN_MD_BLESS=1 cargo nextest run --test seed_history\n\
         Without the entry, projects carrying this version never heal (seeds.rs).",
        missing.join("\n  ")
    );
}

/// The bless, as a pure function: prior manifest text + the lines to add → new text.
///
/// Pure and separate on purpose. The append-only property is the one thing this gate
/// exists to hold, and a property asserted only against a local `String` the test
/// itself built is asserted against nothing. This way `the_bless_appends_…` tests the
/// code the bless path actually runs.
fn append_missing(src: &str, missing: &[String]) -> String {
    let mut out = src.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for line in missing {
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[test]
fn the_bless_appends_and_never_drops_a_line() {
    // The real manifest as input, the real bless function under test, and the
    // assertion is about what it RETURNS — replace the body of `append_missing` with
    // `pack_drift`'s rewrite semantics and this goes red, which is the whole point.
    let src = std::fs::read_to_string(manifest_path()).expect("manifest readable");
    let before = manifest_lines(&src);
    assert!(
        before.len() >= 20,
        "history is substantial: {}",
        before.len()
    );

    let new_line =
        "0000000000000000000000000000000000000000000000000000000000000000  lang/rust.lmd.md"
            .to_string();
    let after = manifest_lines(&append_missing(&src, std::slice::from_ref(&new_line)));

    for line in &before {
        assert!(
            after.contains(line),
            "bless dropped a history line: {}  {}",
            line.0,
            line.1
        );
    }
    assert_eq!(after.len(), before.len() + 1, "bless appends exactly one");
    // Order survives: a bless never reshuffles what is already there, because the
    // heal path reads "current is last" off this file.
    assert_eq!(&after[..before.len()], &before[..], "prior lines unmoved");
    assert_eq!(
        after.last().unwrap().1,
        "lang/rust.lmd.md",
        "new line is last"
    );

    // Nothing to add → byte-identical text. A no-op bless must not touch the file.
    assert_eq!(append_missing(&src, &[]), src, "an empty bless is a no-op");
}
