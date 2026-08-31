//! End-to-end coverage for the two `lean-md mcp` behaviours that live in
//! `cmd_mcp` itself (design 2026-08-31 §2.3 C1 + C3). Both were only ever
//! exercised through in-process seams — `do_render` / `mcp_load_source` for the
//! phase argument, `set_session_sinks_disabled` for the sink kill-switch — so
//! deleting `cmd_mcp`'s own `disable_session_sinks()` call left the whole suite
//! green. These tests drive the real binary over stdio JSON-RPC instead, which
//! is the only place that wiring exists.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_lean-md");

/// A fresh scratch directory. `lean-md mcp` refreshes the project seeds at start
/// and derives the project root from its cwd — inheriting the test runner's cwd
/// would let these tests write into the checkout's own `.lean-ctx/lean-md/`.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lmd_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// `PATH` with `dir` in front, so a stub there shadows the real executable.
fn path_with(dir: &Path) -> std::ffi::OsString {
    let mut path = std::ffi::OsString::from(dir);
    if let Some(existing) = std::env::var_os("PATH") {
        path.push(":");
        path.push(existing);
    }
    path
}

/// Write an executable `lean-ctx` stub into `bin_dir` that appends one line per
/// invocation to `log` and exits 0. `CliBackend` shells out to `lean-ctx` by
/// bare name, so this intercepts every outbound sink call — and makes "no line
/// in the log" mean "no call was made", not "the real binary swallowed it".
fn stub_lean_ctx(bin_dir: &Path, log: &Path) {
    std::fs::create_dir_all(bin_dir).expect("create stub bin dir");
    let stub = bin_dir.join("lean-ctx");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"{}\"\nexit 0\n",
            log.display()
        ),
    )
    .expect("write lean-ctx stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755))
            .expect("chmod lean-ctx stub");
    }
}

fn read_log(log: &Path) -> String {
    std::fs::read_to_string(log).unwrap_or_default()
}

/// Spawn `lean-md mcp` in `cwd`, feed it `requests` (one JSON-RPC object per
/// line), close stdin so the server drains and exits, and return the parsed
/// responses in the order they were written to stdout.
fn mcp_roundtrip_in(cwd: &Path, stub_bin: Option<&Path>, requests: &[Value]) -> Vec<Value> {
    let mut cmd = Command::new(BIN);
    cmd.arg("mcp")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(dir) = stub_bin {
        cmd.env("PATH", path_with(dir));
    }
    let mut child = cmd.spawn().expect("spawn lean-md mcp");

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

fn text_of(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("expected a text result: {response}"))
        .to_string()
}

const TWO_PHASE_PLAN: &str = "@lean-md\nconsumer: ai\n\n\
@phase \"t1\"\nONE\n@phase-end\n\
@phase \"t2\"\nTWO\n@phase-end\n";

#[test]
fn mcp_tools_call_renders_only_the_named_phase_and_refuses_an_unknown_one() {
    // C1 over the wire (spec §3: "MCP `tools/call` mit `path` + `phase`"). The
    // in-process `do_render` test cannot see the argument plumbing in `cmd_mcp`
    // — reading `phase` out of `arguments` and mapping `PhaseNotFound` to the
    // caller-error code — which is where C1 actually lives.
    let cwd = scratch("mcp_phase_e2e");
    let plan = cwd.join("p.lmd.md");
    std::fs::write(&plan, TWO_PHASE_PLAN).expect("write plan");
    let plan_arg = plan.to_str().expect("utf-8 plan path");

    let responses = mcp_roundtrip_in(
        &cwd,
        None,
        &[
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "ctx_md_render",
                            "arguments": { "path": plan_arg, "phase": "t1" } }
            }),
            json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "ctx_md_render",
                            "arguments": { "path": plan_arg, "phase": "nope" } }
            }),
            json!({
                "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": { "name": "ctx_md_render",
                            "arguments": { "content": TWO_PHASE_PLAN, "phase": "t1" } }
            }),
        ],
    );
    assert_eq!(
        responses.len(),
        3,
        "one response per request: {responses:?}"
    );

    let from_path = text_of(&responses[0]);
    assert!(
        from_path.contains("ONE"),
        "named phase missing: {from_path}"
    );
    assert!(
        !from_path.contains("TWO"),
        "the sibling phase must not leak over the wire: {from_path}"
    );

    assert_eq!(
        responses[1]["error"]["code"].as_i64(),
        Some(-32602),
        "an undefined phase is a caller error, not a rendered note: {:?}",
        responses[1]
    );

    let from_content = text_of(&responses[2]);
    assert_eq!(
        from_path, from_content,
        "`path` and `content` must render the same phase identically (#498)"
    );
}

#[test]
fn the_path_branch_refuses_a_companion_instead_of_silently_ignoring_it() {
    // I-6: the `skill` branch rejects `phase` + `companion` with -32602, but the
    // `path`/`content` branch never read `companion` at all — a call carrying it
    // got a full whole-document render back with no hint that the argument was
    // dropped. Same defect class C1 just fixed for `phase`, and the schema text
    // that now advertises both arguments makes it easy to hit.
    let cwd = scratch("mcp_companion_e2e");
    let plan = cwd.join("p.lmd.md");
    std::fs::write(&plan, TWO_PHASE_PLAN).expect("write plan");
    let plan_arg = plan.to_str().expect("utf-8 plan path");

    let responses = mcp_roundtrip_in(
        &cwd,
        None,
        &[
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "ctx_md_render",
                            "arguments": { "path": plan_arg, "companion": "reviewing" } }
            }),
            json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "ctx_md_render",
                            "arguments": { "content": TWO_PHASE_PLAN, "companion": "reviewing" } }
            }),
        ],
    );
    for r in &responses {
        assert_eq!(
            r["error"]["code"].as_i64(),
            Some(-32602),
            "a companion without a skill is a caller error, not a whole-doc render: {r:?}"
        );
        let msg = r["error"]["message"].as_str().unwrap_or_default();
        assert!(
            msg.contains("companion"),
            "the error must name the offending argument: {msg}"
        );
    }

    // The CLI carried the identical swallow (`--companion` without `--skill`
    // fell through to the whole-document branch), and the project rules send
    // agents to the CLI to verify a render — so both surfaces refuse it.
    let cli = Command::new(BIN)
        .arg("render")
        .arg(&plan)
        .arg("--companion")
        .arg("reviewing")
        .current_dir(&cwd)
        .output()
        .expect("run lean-md render");
    assert!(
        !cli.status.success(),
        "the CLI must refuse a companion without --skill, not render the whole doc: {}",
        String::from_utf8_lossy(&cli.stdout)
    );
    assert!(
        String::from_utf8_lossy(&cli.stderr).contains("--companion requires --skill"),
        "stderr must name the offending flag: {}",
        String::from_utf8_lossy(&cli.stderr)
    );
}

#[test]
fn a_render_error_note_cannot_break_out_of_its_own_html_comment() {
    // M-6: `format!("<!-- lmd render error: {e:?} -->")` interpolates
    // `SkillRenderError::DuplicatePhase`, which carries the phase name straight
    // from the source document. A name containing `-->` closes the comment early
    // and the rest of the message lands in the rendered body as content — the
    // breakout Task 2 hardened `render::sanitize_comment` against everywhere
    // else.
    let cwd = scratch("mcp_note_breakout_e2e");
    let breakout = "@phase \"a-->b<!--c\"\nONE\n@phase-end\n\
                    @phase \"a-->b<!--c\"\nTWO\n@phase-end\n";

    let responses = mcp_roundtrip_in(
        &cwd,
        None,
        &[json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "ctx_md_render", "arguments": { "content": breakout } }
        })],
    );
    let text = text_of(&responses[0]);
    assert_eq!(
        text.matches("-->").count(),
        1,
        "the note must close exactly once — its own delimiter: {text:?}"
    );
    assert!(
        text.trim_end().ends_with("-->"),
        "the note must end with its own delimiter: {text:?}"
    );
    assert!(
        text.contains("a--&gt;b&lt;!--c"),
        "the phase name must be escaped, not dropped: {text:?}"
    );
}

#[test]
fn the_mcp_server_silences_the_sinks_while_the_cli_still_fires_them() {
    // C3 over the wire. The unit test flips the kill-switch itself through the
    // test seam, so it stays green even with `cmd_mcp`'s own
    // `disable_session_sinks()` deleted — the recursion guard would silently be
    // gone in the only mode that needs it. Here the switch is only ever thrown
    // by the server's own start-up.
    let cwd = scratch("mcp_sinks_e2e");
    let stub_bin = cwd.join("stub-bin");
    let log = cwd.join("lean-ctx-calls.log");
    stub_lean_ctx(&stub_bin, &log);

    let plan = cwd.join("p.lmd.md");
    std::fs::write(
        &plan,
        "@lean-md\nconsumer: ai\n\n\
         @phase \"t1\"\nBODY\n@on complete decision=\"t1 done\"\n@phase-end\n",
    )
    .expect("write plan");
    let plan_arg = plan.to_str().expect("utf-8 plan path");

    // Control — the CLI keeps its sinks: `lean-ctx call ctx_session …` reaches
    // the stub. Two things ride on this half. It proves the stub is genuinely on
    // the child's PATH, so an empty log in the MCP half means "not called"
    // rather than "never wired up"; and it proves an isolated `--phase` render
    // still opens a scope its `@on complete` can fire from at all.
    let cli = Command::new(BIN)
        .arg("render")
        .arg(&plan)
        .arg("--phase")
        .arg("t1")
        .current_dir(&cwd)
        .env("PATH", path_with(&stub_bin))
        .output()
        .expect("run lean-md render");
    assert!(
        cli.status.success(),
        "cli render failed: {}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let cli_log = read_log(&log);
    assert!(
        cli_log.contains("ctx_session"),
        "the CLI path must still fire its session sink: {cli_log:?}"
    );

    // MCP mode: the same plan, the same stub, no outbound call at all.
    let _ = std::fs::remove_file(&log);
    let responses = mcp_roundtrip_in(
        &cwd,
        Some(&stub_bin),
        &[json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "ctx_md_render",
                        "arguments": { "path": plan_arg, "phase": "t1" } }
        })],
    );
    let rendered = text_of(&responses[0]);
    assert!(
        rendered.contains("BODY"),
        "the phase body must still render in MCP mode: {rendered}"
    );
    let mcp_log = read_log(&log);
    assert!(
        mcp_log.is_empty(),
        "MCP server mode must not call back into lean-ctx — that is the gateway \
         recursion C3 removes: {mcp_log:?}"
    );
}
