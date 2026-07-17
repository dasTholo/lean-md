//! `lean-md check` as a BINARY: the exit code is the contract a CI gate reads.
//!
//! Every other `check` test asserts on the STRING `do_check` returns — which is why a
//! reporting verb could print "lmd errors:" and still exit 0 for a whole package. These
//! tests drive the real process and assert the code.
use std::process::Command;

fn run(args: &[&str], cwd: &std::path::Path) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_lean-md"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run lean-md");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// cwd is PINNED to a fresh temp dir — never the repo. `check` is read-only (D-1), but a
/// binary that resolves its project root from cwd must never be pointed at the real tree.
fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("lmd_check_exit_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_broken_file_exits_nonzero() {
    // The defect class of this package, in the verb that reports it: a CI gate running
    // `lean-md check plan.lmd.md` on a file with a duplicate @phase and a swallowed
    // @dispatch arg went GREEN. `--list-phases` answers the same source with exit 1.
    let dir = scratch("broken");
    let plan = dir.join("broken.lmd.md");
    std::fs::write(
        &plan,
        "@lean-md\nconsumer: ai\n\n@dispatch brief=x phase=y\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n",
    )
    .unwrap();
    let (stdout, stderr, code) = run(&["check", plan.to_str().unwrap()], &dir);
    assert_eq!(code, 1, "a file with errors must not exit 0: {stderr}");
    assert!(
        stderr.contains("lmd errors:"),
        "errors are diagnostics — stderr, as in --list-phases: {stderr}"
    );
    assert!(stderr.contains("unknown argument"), "{stderr}");
    assert!(stderr.contains("duplicate @phase"), "{stderr}");
    assert!(
        !stdout.contains("lmd errors"),
        "no error text may reach stdout: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_clean_file_still_exits_zero() {
    let dir = scratch("clean");
    let plan = dir.join("clean.lmd.md");
    std::fs::write(
        &plan,
        "@lean-md\nconsumer: ai\n\n@dispatch phase=t role=review\n",
    )
    .unwrap();
    let (stdout, stderr, code) = run(&["check", plan.to_str().unwrap()], &dir);
    assert_eq!(code, 0, "a clean file must stay green: {stderr}");
    assert!(stdout.contains("lmd ok"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_stays_read_only() {
    // D-1: `check` reports, it never writes. Only `mcp` / `skill install` / `ack` do.
    let dir = scratch("pure");
    let plan = dir.join("clean.lmd.md");
    std::fs::write(&plan, "@lean-md\nconsumer: ai\n\nhi\n").unwrap();
    let (_o, _e, code) = run(&["check", plan.to_str().unwrap()], &dir);
    assert_eq!(code, 0);
    assert!(
        !dir.join(".lean-ctx").exists(),
        "check must not materialize a project tree"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
