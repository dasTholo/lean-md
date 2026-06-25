//! #3: the same code-intel directive routed via `CliBackend` and `McpBackend`
//! returns byte-identical output. Requires a reachable lean-ctx endpoint
//! (`LEAN_MD_MCP_ENDPOINT`) + `lean-ctx` in PATH; otherwise `#[ignore]`'d so CI
//! that lacks a live endpoint still compiles + skips it cleanly.
#![cfg(feature = "mcp")]

use lean_md::backend::mcp::McpBackend;
use lean_md::backend::{CliBackend, CodeIntelBackend};

#[test]
#[ignore = "needs LEAN_MD_MCP_ENDPOINT + lean-ctx in PATH"]
fn cli_and_mcp_byte_identical_for_ctx_tree() {
    let root = std::env::temp_dir().join("lean_md_parity");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("P.txt"), b"x").unwrap();
    let root_s = root.to_string_lossy().into_owned();

    let cli = CliBackend {
        project_root: root_s.clone(),
    };
    let endpoint = std::env::var("LEAN_MD_MCP_ENDPOINT").expect("LEAN_MD_MCP_ENDPOINT");
    // Task 5: McpBackend::new validates the endpoint URL and returns a Result.
    let mcp = McpBackend::new(endpoint, root_s).expect("mcp backend connect");

    let a = cli
        .call("ctx_tree", serde_json::json!({ "path": "." }))
        .unwrap();
    let b = mcp
        .call("ctx_tree", serde_json::json!({ "path": "." }))
        .unwrap();
    assert_eq!(a, b, "Cli/Mcp byte-parity broken");
}
