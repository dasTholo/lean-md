//! `lmd_render`/`lmd_check` are additive MCP tool-name aliases for
//! `ctx_md_render`/`ctx_md_check` (Task 2, render-call-convention). Proves the
//! `tools/call` dispatch is byte-identical for identical `arguments` (#498) —
//! not just "both respond without error".
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_lean-md");

/// Spawn `lean-md mcp`, feed it `requests` (one JSON-RPC object per line),
/// close stdin so the server drains and exits, and return the parsed
/// responses in the order they were written to stdout.
fn mcp_roundtrip(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(BIN)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn lean-md mcp");

    let mut stdin = child.stdin.take().expect("child stdin");
    for req in requests {
        writeln!(stdin, "{req}").expect("write mcp request");
    }
    drop(stdin); // EOF → cmd_mcp's stdin loop drains and returns

    let stdout = child.stdout.take().expect("child stdout");
    let responses: Vec<Value> = BufReader::new(stdout)
        .lines()
        .map(|l| l.expect("read mcp response line"))
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(&l).expect("parse mcp response json"))
        .collect();

    let status = child.wait().expect("wait lean-md mcp");
    assert!(status.success(), "lean-md mcp must exit 0");
    responses
}

#[test]
fn lmd_render_and_ctx_md_render_dispatch_identically() {
    let content = "@lean-md\nconsumer: ai\n\nHello from the alias parity test.\n";
    let requests = vec![
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "ctx_md_render", "arguments": { "content": content } }
        }),
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "lmd_render", "arguments": { "content": content } }
        }),
    ];
    let responses = mcp_roundtrip(&requests);
    assert_eq!(
        responses.len(),
        2,
        "both tools/call requests must get a response: {responses:?}"
    );
    assert!(
        responses[0].get("error").is_none() && responses[1].get("error").is_none(),
        "neither dispatch may error: {responses:?}"
    );
    assert_eq!(
        responses[0]["result"], responses[1]["result"],
        "lmd_render must dispatch byte-identically to ctx_md_render (#498) for identical arguments: {responses:?}"
    );
}

#[test]
fn lmd_check_and_ctx_md_check_dispatch_identically() {
    let content = "@lean-md\nconsumer: ai\ncrp: compact\n\n@var x default=\"y\"\n";
    let requests = vec![
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "ctx_md_check", "arguments": { "content": content } }
        }),
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "lmd_check", "arguments": { "content": content } }
        }),
    ];
    let responses = mcp_roundtrip(&requests);
    assert_eq!(
        responses.len(),
        2,
        "both tools/call requests must get a response: {responses:?}"
    );
    assert!(
        responses[0].get("error").is_none() && responses[1].get("error").is_none(),
        "neither dispatch may error: {responses:?}"
    );
    assert_eq!(
        responses[0]["result"], responses[1]["result"],
        "lmd_check must dispatch byte-identically to ctx_md_check (#498) for identical arguments: {responses:?}"
    );
}
