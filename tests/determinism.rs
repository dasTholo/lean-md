//! #7: built-in seeds are byte-identical to the on-disk content/core/ seeds, and
//! repeated renders of the same input are byte-stable (#498, no timestamps).
use lean_md::FragmentRegistry;
use std::path::Path;

#[test]
fn builtin_seeds_match_disk() {
    let core = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("content")
        .join("core");
    let reg = FragmentRegistry::with_builtins();
    for name in ["hard-rules", "dispatch-contract"] {
        let disk = std::fs::read_to_string(core.join(format!("{name}.lmd.md"))).unwrap();
        let builtin = reg.resolve(name, Path::new(".")).unwrap();
        assert_eq!(builtin, disk, "{name} drifted from seed file");
    }
}

#[test]
fn render_is_byte_stable_across_runs() {
    let input = "@if consumer=claude\nstable {{ consumer }}\n@if-end\n";
    let a = lean_md::render(input);
    let b = lean_md::render(input);
    assert_eq!(a, b, "render must be deterministic (no timestamp/counter)");
}
