//! SDD recipes expand to the right directives/sinks (design section "Neue Directive-Bridges").
use std::process::Command;

fn render_from_repo(plan_body: &str) -> String {
    // Run from a temp project root with the real recipe library copied in, so
    // `@import .lean-ctx/lean-md/plan-recipes /` resolves (same path real plans use).
    let dir = std::env::temp_dir().join(format!("lmd_sddrec_{}", std::process::id()));
    let recipes_dir = dir.join(".lean-ctx/lean-md");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&recipes_dir).unwrap();
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/content/templates/plan-recipes.lmd.md"
        ),
        recipes_dir.join("plan-recipes.lmd.md"),
    )
    .unwrap();
    std::fs::create_dir_all(recipes_dir.join("lang")).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/content/lang/rust.lmd.md"),
        recipes_dir.join("lang/rust.lmd.md"),
    )
    .unwrap();
    let plan = dir.join("p.lmd.md");
    std::fs::write(&plan, plan_body).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_lean-md"))
        .args(["render", plan.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .expect("run lean-md");
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn recipe_snapshot_expands_to_checkpoint() {
    let out = render_from_repo(
        "@lean-md 0.4\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n\n@call snapshot(\"pre-task-3\") /\n",
    );
    assert!(out.contains("@checkpoint action=snapshot"), "got: {out}");
    assert!(out.contains("pre-task-3"), "label must survive: {out}");
}

#[test]
fn recipe_compress_expands() {
    let out = render_from_repo(
        "@lean-md 0.4\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n\n@call compress() /\n",
    );
    assert!(out.contains("@compress action=checkpoint"), "got: {out}");
}
