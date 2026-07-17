//! `.lean-ctx/lean-md.lock` — seed provenance.
//!
//! `materialize_contracts` can only see "local != embedded" and cannot tell a user
//! edit from a seed that moved on. The lock preserves the *historical* seed hash —
//! the one a newer binary no longer carries. No hash, no provenance.
//!
//! Format is coreutils `sha256sum`, not TOML: the semantics of `sha256sum -c`
//! *are* the provenance question, so the user can answer it without trusting us.
//! Paths are relative to `.lean-ctx/`, i.e. the directory `sha256sum -c` runs in.
//!
//! # Two channels, one file: provenance and consent
//!
//! Entries answer "what did WE last write here" (provenance). Ack lines answer
//! "which embedded seed has the user already seen and declined" (consent). They must
//! stay separate: adopting a local edit as provenance would silence the report at the
//! price of disabling healing for that seed forever.
//!
//! An ack's value is the *embedded* hash, while the file on disk carries the user's
//! own bytes. So an ack can never be written as a `<hex>  <path>` line — not here and
//! not in a `sha256sum`-shaped sidecar either: it is not a claim about a file on disk,
//! and coreutils would dutifully report FAILED for a file that is exactly as the user
//! wants it. The `#` comment channel is the only place in this format where a hash may
//! live without being such a claim (`sha256sum -c` skips `#` lines — see
//! `lock_is_checkable_by_coreutils_sha256sum`, which asserts it against the real tool).
//! Keeping the ack in the lock rather than a sidecar also keeps the two answers about
//! one key travelling together through a commit, a revert or a restore.

use std::path::{Path, PathBuf};

/// Lock location, relative to the project root.
pub const LOCK_REL: &str = ".lean-ctx/lean-md.lock";

/// Comment prefix carrying the consent channel: `# acked <key> <hex>`.
const ACK_PREFIX: &str = "# acked ";

/// Seed provenance plus the user's standing consent.
///
/// `entries` — (path relative to `.lean-ctx/`, lowercase hex digest of what we wrote).
/// `acks` — (same key, hex digest of the *embedded* seed the user acknowledged).
#[derive(Default)]
pub struct Lock {
    entries: Vec<(String, String)>,
    acks: Vec<(String, String)>,
}

impl Lock {
    /// Reads the lock; an absent or unreadable lock is an empty lock, never an error.
    pub fn load(project_root: &Path) -> Lock {
        let mut lock = Lock::default();
        let Ok(raw) = std::fs::read_to_string(Self::path(project_root)) else {
            return lock;
        };
        for line in raw.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            // `# acked <key> <hex>` — our own comment namespace. coreutils skips every
            // `#` line, so this rides along without ever becoming a checksum claim.
            if let Some(rest) = line.strip_prefix(ACK_PREFIX) {
                let mut it = rest.split_whitespace();
                if let (Some(rel), Some(hex), None) = (it.next(), it.next(), it.next()) {
                    lock.set_ack(rel, hex);
                }
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            if let Some((hex, rel)) = line.split_once("  ") {
                lock.set(rel, hex);
            }
        }
        lock
    }

    pub fn get(&self, rel: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(p, _)| p == rel)
            .map(|(_, h)| h.as_str())
    }

    pub fn set(&mut self, rel: &str, hex: &str) {
        match self.entries.iter_mut().find(|(p, _)| p == rel) {
            Some(e) => e.1 = hex.to_string(),
            None => self.entries.push((rel.to_string(), hex.to_string())),
        }
    }

    /// The embedded-seed hash the user has acknowledged for `rel`, if any.
    pub fn ack(&self, rel: &str) -> Option<&str> {
        self.acks
            .iter()
            .find(|(p, _)| p == rel)
            .map(|(_, h)| h.as_str())
    }

    /// Record consent for exactly this embedded seed. Consent is for THIS proposal:
    /// once the seed moves on, `hex` no longer matches and the report speaks again.
    pub fn set_ack(&mut self, rel: &str, hex: &str) {
        match self.acks.iter_mut().find(|(p, _)| p == rel) {
            Some(e) => e.1 = hex.to_string(),
            None => self.acks.push((rel.to_string(), hex.to_string())),
        }
    }

    /// Drop consent for `rel`; returns whether anything was there. Called once the
    /// conflict is gone (reverted or healed) — a spent ack left behind would silence
    /// the *next*, unrelated edit.
    pub fn clear_ack(&mut self, rel: &str) -> bool {
        let n = self.acks.len();
        self.acks.retain(|(p, _)| p != rel);
        self.acks.len() != n
    }

    /// `sha256sum` rendering. Entries and acks are each sorted by path so the bytes
    /// are a function of the content alone (#498), not of insertion order.
    pub fn render(&self) -> String {
        let mut sorted = self.entries.clone();
        sorted.sort();
        let mut acks = self.acks.clone();
        acks.sort();
        let mut out = String::from(
            "# lean-md.lock — generated by lean-md; commit this file.\n\
             # binary_version: ",
        );
        out.push_str(env!("CARGO_PKG_VERSION"));
        out.push_str("\n# Eigene Anpassungen prüfen:  cd .lean-ctx && sha256sum -c lean-md.lock\n");
        for (rel, hex) in &acks {
            out.push_str(&format!("{ACK_PREFIX}{rel} {hex}\n"));
        }
        for (rel, hex) in &sorted {
            out.push_str(&format!("{hex}  {rel}\n"));
        }
        out
    }

    pub fn save(&self, project_root: &Path) -> std::io::Result<()> {
        let path = Self::path(project_root);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.render())
    }

    fn path(project_root: &Path) -> PathBuf {
        project_root.join(LOCK_REL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_checkable_by_coreutils_sha256sum() {
        // The whole reason SHA-256 was chosen over a dep-free hash: the user must be able
        // to re-check every value with a standard command. If coreutils cannot read our
        // file, the format has failed its only job. Also verifies the spec's unverified
        // assumption that `#` lines are ignored by --check.
        let root = std::env::temp_dir().join(format!("lmd_lock_c14n_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("probe.lmd.md"), "probe content\n").unwrap();

        let mut lock = Lock::load(&root);
        lock.set(
            "lean-md/probe.lmd.md",
            &crate::hashx::sha256_hex(b"probe content\n"),
        );
        lock.save(&root).unwrap();

        let out = std::process::Command::new("sha256sum")
            .arg("-c")
            .arg("lean-md.lock")
            .current_dir(root.join(".lean-ctx"))
            .output();
        let Ok(out) = out else {
            eprintln!("sha256sum unavailable — skipping coreutils cross-check");
            let _ = std::fs::remove_dir_all(&root);
            return;
        };
        assert!(
            out.status.success(),
            "coreutils rejected our lock: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        // An edited seed must read as FAILED — that is the provenance question itself.
        std::fs::write(dir.join("probe.lmd.md"), "user edit\n").unwrap();
        let out = std::process::Command::new("sha256sum")
            .arg("-c")
            .arg("lean-md.lock")
            .current_dir(root.join(".lean-ctx"))
            .output()
            .unwrap();
        assert!(!out.status.success(), "edited seed must fail sha256sum -c");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_lock_stays_checkable_by_coreutils_after_an_ack() {
        // sha256sum -c is the format's only job. An ack must not break it. The ack's
        // value is the EMBEDDED hash while the file on disk holds the user's bytes, so
        // written as a checksum line it would make coreutils report FAILED for a file
        // that is exactly as intended. It rides in the `#` comment channel instead.
        let root = std::env::temp_dir().join(format!("lmd_lock_ack_c14n_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("probe.lmd.md"), "probe content\n").unwrap();

        let mut lock = Lock::load(&root);
        lock.set(
            "lean-md/probe.lmd.md",
            &crate::hashx::sha256_hex(b"probe content\n"),
        );
        // A conflicted, acknowledged seed: consent names a hash no file on disk carries.
        lock.set_ack(
            "lean-md/edited.lmd.md",
            &crate::hashx::sha256_hex(b"an embedded seed the user declined\n"),
        );
        lock.save(&root).unwrap();

        let out = std::process::Command::new("sha256sum")
            .arg("-c")
            .arg("lean-md.lock")
            .current_dir(root.join(".lean-ctx"))
            .output();
        let Ok(out) = out else {
            eprintln!("sha256sum unavailable — skipping coreutils cross-check");
            let _ = std::fs::remove_dir_all(&root);
            return;
        };
        assert!(
            out.status.success(),
            "an ack must stay invisible to coreutils: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        // And no phantom file was implied by the ack line.
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains("edited.lmd.md"),
            "the ack line must not make coreutils look for a file"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn acks_round_trip_and_are_a_separate_channel_from_provenance() {
        let root = std::env::temp_dir().join(format!("lmd_lock_ack_rt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();

        let mut lock = Lock::load(&root);
        lock.set("lean-md/lang/rust.lmd.md", "aaaa");
        lock.set_ack("lean-md/lang/rust.lmd.md", "bbbb");
        lock.save(&root).unwrap();

        let raw = std::fs::read_to_string(root.join(".lean-ctx/lean-md.lock")).unwrap();
        assert!(
            raw.contains("# acked lean-md/lang/rust.lmd.md bbbb"),
            "ack must be a `#` comment, not a checksum line: {raw}"
        );

        let back = Lock::load(&root);
        assert_eq!(back.get("lean-md/lang/rust.lmd.md"), Some("aaaa"));
        assert_eq!(back.ack("lean-md/lang/rust.lmd.md"), Some("bbbb"));

        let mut back = back;
        assert!(back.clear_ack("lean-md/lang/rust.lmd.md"));
        assert_eq!(back.ack("lean-md/lang/rust.lmd.md"), None);
        assert_eq!(
            back.get("lean-md/lang/rust.lmd.md"),
            Some("aaaa"),
            "clearing consent must not touch provenance"
        );
        assert!(!back.clear_ack("lean-md/lang/rust.lmd.md"));
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn absent_lock_loads_empty_and_is_not_an_error() {
        let root = std::env::temp_dir().join(format!("lmd_lock_absent_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let lock = Lock::load(&root);
        assert_eq!(lock.get("lean-md/lang/rust.lmd.md"), None);
    }

    #[test]
    fn lock_round_trips_and_paths_are_relative_to_lean_ctx() {
        let root = std::env::temp_dir().join(format!("lmd_lock_rt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        let mut lock = Lock::load(&root);
        lock.set("lean-md/lang/rust.lmd.md", "deadbeef");
        lock.save(&root).unwrap();

        let raw = std::fs::read_to_string(root.join(".lean-ctx/lean-md.lock")).unwrap();
        assert!(
            raw.contains("deadbeef  lean-md/lang/rust.lmd.md"),
            "sha256sum format (two spaces), path relative to .lean-ctx: {raw}"
        );
        assert!(
            raw.contains("# binary_version: "),
            "binary_version comment missing"
        );
        assert!(
            !raw.contains(".lean-ctx/lean-md/lang"),
            "paths must NOT be relative to project_root — sha256sum -c runs in .lean-ctx"
        );
        assert_eq!(
            Lock::load(&root).get("lean-md/lang/rust.lmd.md"),
            Some("deadbeef")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lock_render_is_byte_stable() {
        // #498: same entries → same bytes, regardless of insertion order.
        let mut a = Lock::default();
        a.set("lean-md/b.lmd.md", "22");
        a.set("lean-md/a.lmd.md", "11");
        let mut b = Lock::default();
        b.set("lean-md/a.lmd.md", "11");
        b.set("lean-md/b.lmd.md", "22");
        assert_eq!(a.render(), b.render());
    }
}
