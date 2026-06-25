//! #4: after `lean-ctx addon add ./lean-ctx-addon.toml`, ctx_md_render is
//! reachable via the lean-ctx server AND byte-identical to a direct
//! `lean-md mcp` ctx_md_render call. Requires both binaries installed.
use std::process::Command;

fn direct_render(input_path: &str) -> String {
    let out = Command::new("lean-md")
        .args(["render", input_path])
        .output()
        .expect("lean-md render");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn via_leanctx_call(input_path: &str, root: &str) -> String {
    // ctx_md_render reached through the lean-ctx server/gateway after install.
    let args = format!(r#"{{"path": "{input_path}"}}"#);
    let out = Command::new("lean-ctx")
        .args([
            "call",
            "ctx_md_render",
            "--project-root",
            root,
            "--json",
            &args,
        ])
        .output()
        .expect("lean-ctx call ctx_md_render");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
#[ignore = "needs lean-ctx + lean-md installed and addon added"]
fn addon_render_matches_direct_render() {
    let dir = std::env::temp_dir().join("lean_md_roundtrip");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("doc.lmd.md");
    std::fs::write(&f, "@date\nroundtrip marker\n").unwrap();
    let path = f.to_str().unwrap();
    let root = dir.to_str().unwrap();

    let direct = direct_render(path);
    let via = via_leanctx_call(path, root);
    assert_eq!(
        direct, via,
        "addon-path render must equal direct render (#4)"
    );
}
