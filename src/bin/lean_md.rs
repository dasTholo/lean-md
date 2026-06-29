//! `lean-md` — standalone CLI: render/check/mcp `.lmd.md` sources.
//! Mirrors the `lean-ctx md <render|check>` subcommand against the decoupled
//! `lean_md` library crate (no lean-ctx engine linkage). Code-intel directives
//! degrade to the `BACKEND_REQUIRED` envelope unless `lean-ctx` is on PATH
//! (the `CliBackend` default shells out to it).
//!
//! Subcommands:
//!   render <file> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]
//!   check  <file>
//!   mcp              — stdio JSON-RPC 2.0 MCP server (line-delimited framing)

use lean_md::crp_proto::CrpMode;
use lean_md::engine::render_with_overrides;
use lean_md::header::{Consumer, parse_header};
use lean_md::skills::render_skill;
use serde_json::{Value, json};

// ─── Shared helpers ────────────────────────────────────────────────────────

/// Load source text from a file path.  Returns (source, jail_root).
fn load_file(path: &str) -> (String, std::path::PathBuf) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lean-md: read {path}: {e}");
            std::process::exit(1);
        }
    };
    let jail = std::path::Path::new(path).parent().map_or_else(
        || std::path::PathBuf::from("."),
        std::path::Path::to_path_buf,
    );
    (source, jail)
}

/// Core render logic — shared by `cmd_render` and the MCP `ctx_md_render` handler.
fn do_render(
    source: &str,
    jail: std::path::PathBuf,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
) -> String {
    render_with_overrides(source, consumer, crp, jail)
}

/// Core check logic — shared by `cmd_check` and the MCP `ctx_md_check` handler.
fn do_check(source: &str) -> String {
    let (header, body) = parse_header(source);
    let directives = body
        .lines()
        .filter(|l| l.trim_start().starts_with('@'))
        .count();
    format!(
        "lmd ok — consumer={:?}, crp={:?}, directives={}",
        header.consumer, header.crp, directives
    )
}

// ─── CLI flags ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, PartialEq)]
struct RenderArgs {
    file: Option<String>,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    out: Option<String>,
    skill: Option<String>,
    phase: Option<String>,
}

fn parse_render_flags(rest: &[String]) -> RenderArgs {
    let mut a = RenderArgs::default();
    let mut i = 0;
    while i < rest.len() {
        let arg = rest[i].as_str();
        match arg {
            _ if arg.starts_with("--consumer=") => {
                a.consumer = match arg.trim_start_matches("--consumer=") {
                    "human" => Some(Consumer::Human),
                    "ai" => Some(Consumer::Ai),
                    _ => None,
                };
            }
            _ if arg.starts_with("--crp=") => {
                a.crp = CrpMode::parse(arg.trim_start_matches("--crp="));
            }
            "-o" | "--out" => {
                i += 1;
                a.out = rest.get(i).cloned();
            }
            "--skill" => {
                i += 1;
                a.skill = rest.get(i).cloned();
            }
            "--phase" => {
                i += 1;
                a.phase = rest.get(i).cloned();
            }
            _ if !arg.starts_with('-') && a.file.is_none() => a.file = Some(arg.to_string()),
            _ => {}
        }
        i += 1;
    }
    a
}

// ─── main ──────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let action = args.first().map_or("help", String::as_str);
    match action {
        "render" => cmd_render(&args[1..]),
        "check" => cmd_check(&args[1..]),
        "mcp" => cmd_mcp(),
        _ => {
            eprintln!(
                "Usage: lean-md <render|check|mcp> [args]\n\
                 \n  render <file.lmd.md> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]\
                 \n  check  <file.lmd.md>\
                 \n  mcp                   (stdio JSON-RPC 2.0 MCP server)"
            );
            std::process::exit(1);
        }
    }
}

// ─── render subcommand ─────────────────────────────────────────────────────

fn cmd_render(rest: &[String]) {
    let a = parse_render_flags(rest);
    if let Some(skill) = a.skill.as_deref() {
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match render_skill(skill, a.phase.as_deref(), a.consumer, a.crp, jail) {
            Ok(rendered) => match a.out {
                Some(out) => {
                    if let Err(e) = std::fs::write(&out, &rendered) {
                        eprintln!("lean-md render: write {out}: {e}");
                        std::process::exit(1);
                    }
                }
                None => print!("{rendered}"),
            },
            Err(e) => {
                eprintln!("lean-md render: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let Some(file) = a.file else {
        eprintln!("lean-md render: missing <file.lmd.md>");
        std::process::exit(1);
    };
    let (source, jail) = load_file(&file);
    let rendered = do_render(&source, jail, a.consumer, a.crp);
    match a.out {
        Some(out) => {
            if let Err(e) = std::fs::write(&out, &rendered) {
                eprintln!("lean-md render: write {out}: {e}");
                std::process::exit(1);
            }
        }
        None => print!("{rendered}"),
    }
}

// ─── check subcommand ──────────────────────────────────────────────────────

fn cmd_check(rest: &[String]) {
    let Some(file) = rest.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("lean-md check: missing <file.lmd.md>");
        std::process::exit(1);
    };
    let (source, _jail) = load_file(file);
    println!("{}", do_check(&source));
}

// ─── mcp subcommand ────────────────────────────────────────────────────────
//
// Framing: line-delimited JSON (one JSON object per line, \n terminated).
// This is the simplest framing for a stdio MCP server; compatible with MCP
// clients that speak newline-framed JSON-RPC 2.0.  Content-Length framing
// is NOT used (no HTTP-style headers).
//
// Protocol subset implemented:
//   initialize                → serverInfo + capabilities.tools
//   notifications/initialized → (no response — it is a notification)
//   tools/list                → list of ctx_md_render + ctx_md_check
//   tools/call                → dispatch to do_render / do_check
//   <unknown>                 → JSON-RPC error -32601 (method not found)

/// Tool input schemas — byte-identical to ctx_md.rs field names and types.
fn tool_defs() -> Value {
    json!([
        {
            "name": "ctx_md_render",
            "description": "Render an lmd (.lmd.md) plan/spec to Markdown. consumer=human narrates \
    directives as prose (readable plan); consumer/crp override the source header.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path":     { "type": "string", "description": "Path to a .lmd.md source" },
                    "content":  { "type": "string", "description": "Inline lmd source (alternative to path)" },
                    "consumer": { "type": "string", "description": "Override audience: ai|human" },
                    "crp":      { "type": "string", "description": "Override CRP mode: tdd|compact|off" },
                    "skill":    { "type": "string", "description": "Render an embedded lmd skill body by name (alternative to path/content)" },
                    "phase":    { "type": "string", "description": "Render only this named phase of the skill (requires skill)" }
                }
            }
        },
        {
            "name": "ctx_md_check",
            "description": "Parse-check an lmd (.lmd.md) source: reports header config and directive \
    count without executing anything.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path":    { "type": "string", "description": "Path to a .lmd.md source" },
                    "content": { "type": "string", "description": "Inline lmd source (alternative to path)" }
                }
            }
        }
    ])
}

/// Resolve lmd source from MCP params: `content` (inline) or `path` (file read).
fn mcp_load_source(params: &Value) -> Result<(String, std::path::PathBuf), String> {
    if let Some(content) = params.get("content").and_then(Value::as_str) {
        return Ok((content.to_string(), std::path::PathBuf::from(".")));
    }
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing 'path' or 'content' parameter".to_string())?;
    let source = std::fs::read_to_string(path).map_err(|e| format!("ctx_md: read {path}: {e}"))?;
    let jail = std::path::Path::new(path).parent().map_or_else(
        || std::path::PathBuf::from("."),
        std::path::Path::to_path_buf,
    );
    Ok((source, jail))
}

/// Build a JSON-RPC 2.0 success response.
fn rpc_ok(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Build a JSON-RPC 2.0 error response.
fn rpc_err(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_flags_parse_skill_and_phase() {
        let a = parse_render_flags(&[
            "--skill".to_string(),
            "lmd-test-driven-development".to_string(),
            "--phase".to_string(),
            "red".to_string(),
        ]);
        assert_eq!(a.skill.as_deref(), Some("lmd-test-driven-development"));
        assert_eq!(a.phase.as_deref(), Some("red"));
    }

    #[test]
    fn skill_render_is_byte_stable_and_isolated() {
        let jail = std::path::PathBuf::from(".");
        let a = render_skill(
            "lmd-test-driven-development",
            Some("green"),
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        let b = render_skill(
            "lmd-test-driven-development",
            Some("green"),
            None,
            None,
            jail,
        )
        .unwrap();
        assert_eq!(a, b, "render_skill must be deterministic (#498)");
        assert!(a.contains("Verify GREEN"));
        assert!(
            !a.contains("Verify RED"),
            "phase isolation in the exposed path"
        );
    }
}

fn cmd_mcp() {
    use std::io::{BufRead, Write};

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let resp = rpc_err(&Value::Null, -32700, &format!("parse error: {e}"));
                let _ = writeln!(out, "{}", resp);
                let _ = out.flush();
                continue;
            }
        };

        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        // notifications have no "id" field → no response
        let is_notification = req.get("id").is_none();

        let resp = match method {
            "initialize" => rpc_ok(
                &id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "lean-md",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }),
            ),

            "notifications/initialized" => {
                // notification — no response
                continue;
            }

            "tools/list" => rpc_ok(&id, json!({ "tools": tool_defs() })),

            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);

                match name {
                    "ctx_md_render" => {
                        if let Some(skill) = args.get("skill").and_then(Value::as_str) {
                            let phase = args.get("phase").and_then(Value::as_str);
                            let consumer =
                                args.get("consumer")
                                    .and_then(Value::as_str)
                                    .and_then(|s| match s.trim() {
                                        "human" => Some(Consumer::Human),
                                        "ai" => Some(Consumer::Ai),
                                        _ => None,
                                    });
                            let crp = args
                                .get("crp")
                                .and_then(Value::as_str)
                                .and_then(|s| CrpMode::parse(s));
                            let jail = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            match render_skill(skill, phase, consumer, crp, jail) {
                                Ok(rendered) => rpc_ok(
                                    &id,
                                    json!({ "content": [{ "type": "text", "text": rendered }] }),
                                ),
                                Err(e) => rpc_err(&id, -32602, &format!("{e}")),
                            }
                        } else {
                            match mcp_load_source(&args) {
                                Ok((source, jail)) => {
                                    let consumer = args
                                        .get("consumer")
                                        .and_then(Value::as_str)
                                        .and_then(|s| match s.trim() {
                                            "human" => Some(Consumer::Human),
                                            "ai" => Some(Consumer::Ai),
                                            _ => None,
                                        });
                                    let crp = args
                                        .get("crp")
                                        .and_then(Value::as_str)
                                        .and_then(|s| CrpMode::parse(s));
                                    let rendered = do_render(&source, jail, consumer, crp);
                                    rpc_ok(
                                        &id,
                                        json!({
                                            "content": [{ "type": "text", "text": rendered }]
                                        }),
                                    )
                                }
                                Err(e) => rpc_err(&id, -32602, &e),
                            }
                        }
                    }

                    "ctx_md_check" => match mcp_load_source(&args) {
                        Ok((source, _jail)) => {
                            let summary = do_check(&source);
                            rpc_ok(
                                &id,
                                json!({
                                    "content": [{ "type": "text", "text": summary }]
                                }),
                            )
                        }
                        Err(e) => rpc_err(&id, -32602, &e),
                    },

                    other => rpc_err(&id, -32601, &format!("unknown tool: {other}")),
                }
            }

            other => {
                if is_notification {
                    continue;
                }
                rpc_err(&id, -32601, &format!("method not found: {other}"))
            }
        };

        let _ = writeln!(out, "{}", resp);
        let _ = out.flush();
    }
}
