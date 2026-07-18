/// Every rule the composed contract carries appears exactly once — and none went
/// missing in the dedup. This is what a dispatched subagent actually reads: the
/// built-in plus the `@include hard-rules` expansion.
#[test]
fn the_dispatch_contract_carries_every_rule_exactly_once() {
    let composed = lean_md::render("@include dispatch-contract\n");

    assert_eq!(
        composed.matches("mode=anchored").count(),
        1,
        "the anchored-edit rule appears once:\n{composed}"
    );
    assert_eq!(
        composed.matches("Never native").count(),
        1,
        "the native-tool ban is stated once, by hard-rules:\n{composed}"
    );

    assert!(composed.contains("fresh"), "the NEVER-fresh rule survives");
    assert!(
        composed.contains("NEVER `raw` on ctx_read"),
        "the unconditional raw ban for ctx_read survives"
    );
    assert!(
        composed.contains("ctx_delta") || composed.contains("mode=diff"),
        "the re-read path stays"
    );

    assert!(
        !composed.contains("(*.rs)"),
        "the edit rule must not read as Rust-only"
    );

    assert!(composed.contains("tool_profile=power"));
    assert!(composed.contains("ToolSearch"));
    assert!(composed.contains("NEVER a heredoc"));
}
