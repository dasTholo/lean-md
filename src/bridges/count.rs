//! `@count` Router bridge → glob match count (Spec §3.1 trivial-R).
//! Uses stdlib `std::fs::read_dir` — no `glob` crate dep.
use std::path::Path;
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@count <glob>` — number of filesystem paths matching the glob pattern.
/// Supports `*` (single-segment wildcard) and `**` (recursive walk).
/// Paths that cannot be read (e.g. permission-denied during traversal) are
/// silently excluded from the count.
pub struct CountBridge;

impl DirectiveBridge for CountBridge {
    fn name(&self) -> &'static str {
        "count"
    }

    fn execute(
        &self,
        _ctx: &Rc<EngineContext>,
        args: &DirectiveArgs,
    ) -> Result<String, BridgeError> {
        let pattern = args
            .positional(0)
            .ok_or(BridgeError::MissingArg("pattern"))?;
        Ok(glob_count(pattern).to_string())
    }
}

/// Count filesystem entries matching a glob pattern.
/// Splits the pattern into a base directory and a pattern suffix, then walks
/// the filesystem using `std::fs::read_dir`. Supports `*` (single-level) and
/// `**` (recursive). Case-sensitive on all platforms.
fn glob_count(pattern: &str) -> usize {
    // Normalise to forward slashes, then split on '/'.
    let norm = pattern.replace('\\', "/");
    let parts: Vec<&str> = norm.split('/').collect();

    // Find the split point: first segment containing a wildcard.
    let split = parts
        .iter()
        .position(|s| s.contains('*') || s.contains('?'));

    let Some(split) = split else {
        // No wildcards — just check if the exact path exists.
        return if Path::new(pattern).exists() { 1 } else { 0 };
    };

    let base: std::path::PathBuf = if split == 0 {
        Path::new(".").to_path_buf()
    } else {
        // Rejoin with '/' rather than `iter().collect()`: a leading empty
        // segment (absolute path, e.g. "/tmp/x") must keep its root, which
        // `PathBuf::from_iter(["", "tmp"])` would drop, yielding a relative path.
        std::path::PathBuf::from(parts[..split].join("/"))
    };
    let segs: Vec<String> = parts[split..].iter().map(|s| s.to_string()).collect();
    let seg_refs: Vec<&str> = segs.iter().map(String::as_str).collect();
    count_matches(&base, &seg_refs)
}

/// Recursively count entries under `dir` that match the remaining pattern
/// segments. `segs[0]` is matched against direct children; `**` triggers a
/// recursive sub-walk for all remaining segments.
fn count_matches(dir: &Path, segs: &[&str]) -> usize {
    if segs.is_empty() {
        return 0;
    }
    let seg = segs[0];
    let rest = &segs[1..];

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    let mut count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let path = entry.path();

        if seg == "**" {
            // `**` matches zero or more directory levels.
            // Try consuming `**` and matching the rest at this level.
            if !rest.is_empty() {
                count += count_matches(dir, rest);
            }
            // Recurse into subdirectories keeping `**` in play.
            if path.is_dir() {
                count += count_matches(&path, segs);
            }
            // `**` with no rest also matches files directly.
            if rest.is_empty() {
                count += 1;
            }
            continue;
        }

        if !glob_match(seg, &name_str) {
            continue;
        }

        if rest.is_empty() {
            // Matched all segments — count this entry.
            count += 1;
        } else if path.is_dir() {
            count += count_matches(&path, rest);
        }
    }
    count
}

/// Simple glob matching for a single path segment.
/// Supports `*` (any sequence of non-separator chars) and `?` (any single char).
fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_match_chars(&p, &n)
}

fn glob_match_chars(p: &[char], n: &[char]) -> bool {
    match (p.first(), n.first()) {
        (None, None) => true,
        (None, _) => false,
        (Some('*'), _) => {
            // `*` matches zero or more chars (within a single segment).
            // Try consuming 0..=n.len() chars.
            for i in 0..=n.len() {
                if glob_match_chars(&p[1..], &n[i..]) {
                    return true;
                }
            }
            false
        }
        (Some('?'), Some(_)) => glob_match_chars(&p[1..], &n[1..]),
        (Some('?'), None) => false,
        (Some(pc), Some(nc)) => *pc == *nc && glob_match_chars(&p[1..], &n[1..]),
        (Some(_), None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DirectiveArgs;
    use crate::engine::EngineContext;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(
            LeanMdHeader::default(),
            PathBuf::from("."),
        ))
    }

    #[test]
    fn counts_matching_files() {
        let dir = std::env::temp_dir().join("lmd_count_bridge");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..3 {
            std::fs::write(dir.join(format!("c{i}.cnt")), "x").unwrap();
        }
        let args = DirectiveArgs::parse(&format!("{}/*.cnt", dir.to_str().unwrap()));
        let out = CountBridge.execute(&ctx(), &args).unwrap();
        assert_eq!(out, "3", "got: {out}");
    }

    #[test]
    fn missing_pattern_errors() {
        let err = CountBridge
            .execute(&ctx(), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn count_is_registered() {
        assert!(super::super::default_registry().get("count").is_some());
    }
}
