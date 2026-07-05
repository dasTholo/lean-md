//! `render --list-phases`: import-independent phase index (name<TAB>title).
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

#[test]
fn render_list_phases_emits_index() {
    let dir = std::env::temp_dir().join(format!("lmd_listphases_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let plan = dir.join("p.lmd.md");
    std::fs::write(
        &plan,
        "@import .lean-ctx/lean-md/nope /\n@phase \"task-1\"\n## Task 1 — first\n@phase-end\n@phase \"task-2\"\n## Task 2 — second\n@phase-end\n",
    )
    .unwrap();
    let (stdout, _e, code) = run(&["render", plan.to_str().unwrap(), "--list-phases"], &dir);
    assert_eq!(code, 0);
    assert_eq!(stdout, "task-1\tTask 1 — first\ntask-2\tTask 2 — second\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_phases_and_phase_mutually_exclusive() {
    let dir = std::env::temp_dir().join(format!("lmd_listphases_mx_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let plan = dir.join("p.lmd.md");
    std::fs::write(&plan, "@phase \"task-1\"\n## T\n@phase-end\n").unwrap();
    let (_o, stderr, code) = run(
        &[
            "render",
            plan.to_str().unwrap(),
            "--list-phases",
            "--phase",
            "task-1",
        ],
        &dir,
    );
    assert_ne!(code, 0);
    assert!(stderr.contains("mutually exclusive"), "got: {stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_phases_empty_source_is_empty_exit_zero() {
    let dir = std::env::temp_dir().join(format!("lmd_listphases_empty_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let plan = dir.join("p.lmd.md");
    std::fs::write(&plan, "no phases here\n").unwrap();
    let (stdout, _e, code) = run(&["render", plan.to_str().unwrap(), "--list-phases"], &dir);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
    let _ = std::fs::remove_dir_all(&dir);
}
