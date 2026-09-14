//! `outline --json`: phases, @calls, macro signatures and findings as data.
use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str], cwd: &std::path::Path, stdin: Option<&str>) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lean-md"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lean-md");
    if let Some(text) = stdin {
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(text.as_bytes())
            .expect("write stdin");
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait lean-md");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn project(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("lmd_outline_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".lean-ctx/lean-md")).unwrap();
    std::fs::write(
        dir.join(".lean-ctx/lean-md/recipes.lmd.md"),
        "@define route(work, lane, files)\n<!-- route -->\n@define-end\n",
    )
    .unwrap();
    dir
}

const PLAN: &str = "@lean-md\nconsumer: ai\n\n@import .lean-ctx/lean-md/recipes /\n\n@phase \"constraints\"\n## Global Constraints\n@phase-end\n@phase \"task-1\"\n@call route(implement, core, \"a.py b.py\")\n## Task 1: first\n@phase-end\n";

#[test]
fn a_clean_plan_exits_zero_with_json() {
    let dir = project("clean");
    std::fs::write(dir.join("p.lmd.md"), PLAN).unwrap();
    let (stdout, stderr, code) = run(
        &[
            "outline",
            "p.lmd.md",
            "--json",
            "--require-phase",
            "constraints",
        ],
        &dir,
        None,
    );
    assert_eq!(code, 0, "{stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json on stdout");
    assert_eq!(v["errors"], serde_json::json!([]));
    assert_eq!(v["phases"][1]["name"], "task-1");
    assert_eq!(
        v["phases"][1]["calls"][0]["args"],
        serde_json::json!(["implement", "core", "a.py b.py"])
    );
    assert_eq!(
        v["macros"]["route"],
        serde_json::json!(["work", "lane", "files"])
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stdin_gives_the_same_bytes_as_the_file() {
    let dir = project("stdin");
    std::fs::write(dir.join("p.lmd.md"), PLAN).unwrap();
    let (from_file, _, _) = run(&["outline", "p.lmd.md", "--json"], &dir, None);
    let (from_stdin, stderr, code) = run(&["outline", "-", "--json"], &dir, Some(PLAN));
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(from_stdin, from_file);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn findings_exit_one_and_still_print_json() {
    let dir = project("findings");
    let (stdout, _, code) = run(
        &["outline", "-", "--json", "--require-phase", "lanes"],
        &dir,
        Some("@phase \"task-1\"\n@call nope() /\n@phase-end\n"),
    );
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json on stdout");
    let kinds: Vec<&str> = v["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["missing_phase", "unknown_macro"]);
    assert!(v["errors"][0].get("phase").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fenced_quoted_and_empty_calls_survive_the_cli() {
    let dir = project("forms");
    let src = "@define g(a, b)\nx\n@define-end\n@define h()\ny\n@define-end\n@phase \"t\"\n```\n@call nope() /\n```\n@call g(\"x, y\", z) /\n@call h() /\n@phase-end\n";
    let (stdout, stderr, code) = run(&["outline", "-", "--json"], &dir, Some(src));
    assert_eq!(code, 0, "{stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json on stdout");
    let calls = &v["phases"][0]["calls"];
    assert_eq!(calls.as_array().unwrap().len(), 2);
    assert_eq!(calls[0]["args"], serde_json::json!(["x, y", "z"]));
    assert_eq!(calls[1]["args"], serde_json::json!([]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_runs_are_byte_identical() {
    let dir = project("stable");
    std::fs::write(dir.join("p.lmd.md"), PLAN).unwrap();
    let (a, _, _) = run(&["outline", "p.lmd.md", "--json"], &dir, None);
    let (b, _, _) = run(&["outline", "p.lmd.md", "--json"], &dir, None);
    assert_eq!(a, b);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn usage_errors_exit_two_without_json() {
    let dir = project("usage");
    for args in [
        vec!["outline", "p.lmd.md"],
        vec!["outline", "--json"],
        vec!["outline", "missing.lmd.md", "--json"],
        vec!["outline", "p.lmd.md", "--json", "--bogus"],
        vec!["outline", "p.lmd.md", "--json", "--require-phase"],
    ] {
        let (stdout, stderr, code) = run(&args, &dir, None);
        assert_eq!(code, 2, "{args:?}: {stderr}");
        assert_eq!(stdout, "", "{args:?}");
        assert!(
            stderr.starts_with("lean-md outline: "),
            "{args:?}: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
