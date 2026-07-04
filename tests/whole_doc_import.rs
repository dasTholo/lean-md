//! Bug 3 regression: `lean-md render <plan>.lmd.md` WITHOUT `--phase` must
//! resolve `@import` against the working dir (project root) — exactly like the
//! `--phase` path — not against the plan file's parent dir.
use std::process::Command;

#[test]
fn whole_doc_render_resolves_import_from_working_dir() {
    // Materialize a project root with the recipe library, and a plan in a subdir.
    let root = std::env::temp_dir().join(format!("lmd_wholedoc_{}", std::process::id()));
    let recipes_dir = root.join(".lean-ctx/lean-md");
    let plan_dir = root.join("docs/plans");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&recipes_dir).unwrap();
    std::fs::create_dir_all(&plan_dir).unwrap();
    std::fs::write(
        recipes_dir.join("plan-recipes.lmd.md"),
        "@define note(x)\nNOTE:{{ x }}\n@define-end\n",
    )
    .unwrap();
    let plan = plan_dir.join("p.lmd.md");
    std::fs::write(
        &plan,
        "@lean-md 0.4\nconsumer: ai\n\n@import .lean-ctx/lean-md/plan-recipes /\n\n@call note(hello) /\n",
    )
    .unwrap();

    // Run the binary from the project root, WITHOUT --phase.
    let out = Command::new(env!("CARGO_BIN_EXE_lean-md"))
        .args(["render", plan.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .expect("run lean-md");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("NOTE:hello"),
        "whole-doc @import + @call must expand, got: {stdout}"
    );
    assert!(
        !stdout.contains("NotFound") && !stdout.contains("macro not found"),
        "import must resolve whole-doc, got: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
