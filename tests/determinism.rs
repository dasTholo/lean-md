//! #7: `resolve` hands back each embedded seed verbatim, and repeated renders of the
//! same input are byte-stable (#498, no timestamps).
use lean_md::FragmentRegistry;
use std::path::Path;

/// What this gate does NOT test: content drift between an embedded seed and its file.
/// `include_str!` is build-invalidating — editing `content/core/hard-rules.lmd.md` forces
/// a rebuild, the const picks the new bytes up, and both sides of the comparison move
/// together. That property is upheld by the compiler and is untestable here by
/// construction (verified: a deliberate `DRIFT_PROBE` in the seed leaves this GREEN).
///
/// What it DOES test, and what genuinely breaks: the wiring between a fragment NAME and
/// the seed bytes `resolve` returns for it. A mis-mapped `builtins.insert` (copy-paste of
/// the wrong const), or a `base`/`resolve` that trims, re-wraps or otherwise mutates the
/// text on its way out, all land here as a hard failure (verified: pointing "hard-rules"
/// at DISPATCH_CONTRACT turns this RED). The on-disk file is the reference for the bytes
/// each name must yield — not a drift counterpart.
#[test]
fn resolve_returns_each_seed_verbatim() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("content");
    // Jail root without any `<name>.ext.lmd.md`: `resolve` composes a project extension
    // onto the built-in, so a jail root that HAS one (this repo's own checkout, when the
    // test runner's cwd is the crate root) would read as built-in drift. The claim under
    // test is the built-in passthrough — the extension layer is a separate concern.
    let jail_root = std::env::temp_dir().join(format!("lmd_determinism_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&jail_root);
    std::fs::create_dir_all(&jail_root).unwrap();
    let reg = FragmentRegistry::with_builtins();
    // Every name registered in `with_builtins`, each with the seed it must resolve to.
    // Keep in sync when a built-in is added — an unlisted one is an untested one.
    let seeds = [
        ("hard-rules", "core/hard-rules.lmd.md"),
        ("dispatch-contract", "core/dispatch-contract.lmd.md"),
        (
            "parallel-dispatch",
            "core/_fragments/parallel-dispatch.lmd.md",
        ),
    ];
    for (name, rel) in seeds {
        let disk = std::fs::read_to_string(content.join(rel)).unwrap();
        let builtin = reg.resolve(name, &jail_root).unwrap();
        assert_eq!(builtin, disk, "{name} does not resolve to {rel} verbatim");
    }
    let _ = std::fs::remove_dir_all(&jail_root);
}

#[test]
fn render_is_byte_stable_across_runs() {
    let input = "@if consumer=claude\nstable {{ consumer }}\n@if-end\n";
    let a = lean_md::render(input);
    let b = lean_md::render(input);
    assert_eq!(a, b, "render must be deterministic (no timestamp/counter)");
}
