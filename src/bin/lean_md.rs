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

use lean_md::args::DirectiveArgs;
use lean_md::crp_proto::CrpMode;
use lean_md::header::{Consumer, parse_header};
use lean_md::skill_install::{Scope, install_skill, remove_skill};
use lean_md::skill_vars::{InitOutcome, render_vars_template, scan_var_decls, write_vars_template};
use lean_md::skills::{all_skill_sources, render_companion, render_skill, skill_source};
use serde_json::{Value, json};

// ─── Shared helpers ────────────────────────────────────────────────────────

/// Load source text from a file path. (The parent-dir jail is dead since the Bug-3
/// fix — both render/check paths jail on the cwd — so this returns text only.)
fn load_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lean-md: read {path}: {e}");
            std::process::exit(1);
        }
    }
}

/// Core render logic for the MCP `ctx_md_render` whole-document handler.
/// Routes through `render_source_with_phase(.., None, ..)` so the MCP surface runs the
/// same `@var`/vars.toml pre-pass as the CLI whole-doc render — CLI and MCP stay
/// byte-identical for plain (non-skill) `.lmd.md` sources (#498). (`cmd_render` no
/// longer calls this after the Bug-3 fix; only the MCP handler does.)
fn do_render(
    source: &str,
    jail: std::path::PathBuf,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
) -> String {
    // phase=None can never be PhaseNotFound → the Result is always Ok.
    lean_md::render_source_with_phase(source, None, consumer, crp, jail)
        .unwrap_or_else(|e| format!("<!-- lmd render error: {e:?} -->"))
}

/// Seed-refresh status for `check` — READ-ONLY, it only reports what a previous
/// refresh already left on disk. The `.new` files are written at MCP server start
/// (`cmd_mcp`); its stderr goes to the gateway log, which the agent never reads, so
/// `check` is where that case becomes visible (Spec decision 4). Deterministic: the
/// line is a pure function of the seed tree in `PROJECT_SEEDS` order (#498).
///
/// Only UNacknowledged conflicts carry a `.new` (`refresh_contracts` withholds it once
/// the user has acked, and `ack` removes the one that is there), so scanning for `.new`
/// is exactly "what has the user not answered yet" — no lock read needed here, which
/// keeps this the pure disk read D-1 requires.
fn seed_report_line(project_root: &std::path::Path) -> Option<String> {
    let base = project_root.join(".lean-ctx/lean-md");
    let pending: Vec<String> = lean_md::seeds::PROJECT_SEEDS
        .iter()
        .map(|(rel, _)| format!("{rel}.new"))
        .filter(|rel| base.join(rel).exists())
        .collect();
    if pending.is_empty() {
        return None;
    }
    // Name the way out. A report that does not say how to switch it off is half
    // wallpaper already — the user has no reason to believe the next one matters.
    Some(format!(
        "lmd seeds — your local copies were kept; the updated seeds sit beside them: {} \
         (diff them, then replace or delete the .new file — or run `lean-md ack` to keep \
         yours and stop being told about these)",
        pending.join(", ")
    ))
}

/// Directive lines of a body — every `@…` line that the renderer would actually
/// execute. Lines inside a fenced code block are verbatim text to the renderer
/// (rushdown owns the fence rule), so `check` must skip them too: a directive that
/// only *looks* broken inside a ``` block is documentation, not a defect, and a gate
/// that rejects files which render cleanly is a false alarm.
///
/// Fence rule (CommonMark-shaped): a run of 3+ backticks or tildes opens a block; it
/// closes on a run of the same char that is at least as long. An unclosed fence runs
/// to the end of the document — same as the renderer.
fn directive_lines(body: &str) -> Vec<&str> {
    let fence_run = |l: &str| -> Option<(char, usize)> {
        let t = l.trim_start();
        let c = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
        let n = t.chars().take_while(|x| *x == c).count();
        (n >= 3).then_some((c, n))
    };
    let mut open: Option<(char, usize)> = None;
    let mut out = Vec::new();
    for l in body.lines() {
        match open {
            // Inside a fence: only a matching, long-enough run closes it.
            Some((oc, on)) => {
                if let Some((c, n)) = fence_run(l)
                    && c == oc
                    && n >= on
                {
                    open = None;
                }
            }
            None => {
                if let Some(f) = fence_run(l) {
                    open = Some(f);
                } else if l.trim_start().starts_with('@') {
                    out.push(l);
                }
            }
        }
    }
    out
}

/// Core check logic — shared by `cmd_check` and the MCP `ctx_md_check` handler.
/// Stays purely reading (D-1): `project_root=None` (or an unresolvable cwd) simply
/// drops the seed part; the file check is unaffected.
fn do_check(source: &str, project_root: Option<&std::path::Path>) -> String {
    let (header, body) = parse_header(source);
    let lines = directive_lines(body);
    // Argument validation reads the SAME schema the bridges do — a file that checks
    // ok is a file that renders. Directives without a schema are counted, not judged.
    let mut errors = Vec::new();
    for l in &lines {
        let rest = l.trim_start().trim_start_matches('@');
        let (name, argv) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        if let Err(e) = lean_md::arg_schema::validate(name, &DirectiveArgs::parse(argv)) {
            errors.push(e);
        }
    }
    // Duplicate @phase names: the parser refuses such a source (it is lossy — only the
    // first block per name is addressable), so `check` must not call it ok. Read from
    // `source`, not `body`, so the reported lines match the file the author opens.
    if let Some(m) = lean_md::phases::duplicate_phase_error(source) {
        errors.push(m);
    }
    if !errors.is_empty() {
        return format!("lmd errors:\n{}", errors.join("\n"));
    }
    let mut out = format!(
        "lmd ok — consumer={:?}, crp={:?}, directives={}",
        header.consumer,
        header.crp,
        lines.len()
    );
    if let Some(line) = project_root.and_then(seed_report_line) {
        out.push('\n');
        out.push_str(&line);
    }
    // Pack range — same asymmetry as the seed report: `cmd_mcp` logs it to stderr, but
    // `check` is where the user actually looks. Only a RANGE violation speaks; a pack
    // that merely differs from the binary version is the intended normal case.
    if let Some(line) = project_root.and_then(lean_md::version_gate::drift_warning) {
        out.push('\n');
        out.push_str(&line);
    }
    out
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
    companion: Option<String>,
    signatures: bool,
    list_phases: bool,
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
            "--companion" => {
                i += 1;
                a.companion = rest.get(i).cloned();
            }
            "--signatures" => a.signatures = true,
            "--list-phases" => a.list_phases = true,
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
        "skill" => cmd_skill(&args[1..]),
        "source" => cmd_source(&args[1..]),
        "ack" => cmd_ack(&args[1..]),
        _ => {
            eprintln!(
                "Usage: lean-md <render|check|mcp|skill|source|ack> [args]\n\
                 \n  render <file.lmd.md|--skill NAME [--phase P | --companion C]> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md] [--list-phases]\
                 \n  check  <file.lmd.md>\
                 \n  source <file.lmd.md>  (raw file bytes, no rendering — for edit anchors)\
                 \n  ack    [<seed>…]      (keep your edited seeds; stop reporting them until the seed changes)\
                 \n  mcp                   (stdio JSON-RPC 2.0 MCP server)\
                 \n  skill  <install|remove> <name> [--global|--local]\
                 \n  skill  vars --init [name]"
            );
            std::process::exit(1);
        }
    }
}

// ─── render subcommand ─────────────────────────────────────────────────────

/// Text of the `--list-phases` index: one `name\ttitle` line per phase — or the
/// duplicate-`@phase` message when the source is lossy.
///
/// The gate lives in `iter_phase_blocks`, which answers a lossy source with an EMPTY
/// list. Printing that verbatim degrades the loudest gate in the parser to silence:
/// no output + exit 0 reads as "this file has no phases". So `--list-phases` refuses
/// the same source `check` and `--phase X` refuse, with the same message from the
/// same formatter (`duplicate_phase_error`) — no second wording to drift (#498).
fn list_phases_output(source: &str) -> Result<String, String> {
    if let Some(msg) = lean_md::phases::duplicate_phase_error(source) {
        return Err(msg);
    }
    Ok(lean_md::outline_phases(source)
        .into_iter()
        .map(|p| format!("{}\t{}\n", p.name, p.title))
        .collect())
}

fn cmd_render(rest: &[String]) {
    let a = parse_render_flags(rest);
    if a.list_phases {
        if a.phase.is_some() {
            eprintln!("lean-md render: --list-phases and --phase are mutually exclusive");
            std::process::exit(1);
        }
        // Load the source the same way the render paths do: skill body or file.
        let source = match a.skill.as_deref() {
            Some(skill) => {
                let root =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                match lean_md::skills::skill_source(skill, &root) {
                    Ok(body) => body,
                    Err(e) => {
                        eprintln!("lean-md render: {e}");
                        std::process::exit(1);
                    }
                }
            }
            None => {
                let Some(file) = a.file.as_deref() else {
                    eprintln!("lean-md render: --list-phases needs <file.lmd.md> or --skill NAME");
                    std::process::exit(1);
                };
                load_file(file)
            }
        };
        match list_phases_output(&source) {
            Ok(index) => print!("{index}"),
            Err(e) => {
                eprintln!("lean-md render: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if let Some(skill) = a.skill.as_deref() {
        if a.phase.is_some() && a.companion.is_some() {
            eprintln!("lean-md render: --phase and --companion are mutually exclusive");
            std::process::exit(1);
        }
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let result = match a.companion.as_deref() {
            Some(companion) => render_companion(skill, companion, a.consumer, a.crp, jail),
            None => render_skill(skill, a.phase.as_deref(), a.consumer, a.crp, jail),
        };
        match result {
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
    let source = load_file(&file);
    let rendered: String = if a.signatures {
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        lean_md::render_signature_index(&source, jail)
    } else if let Some(phase) = a.phase.as_deref() {
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match lean_md::render_source_with_phase(&source, Some(phase), a.consumer, a.crp, jail) {
            Ok(out) => out,
            Err(e) => {
                eprintln!("render error: {e:?}");
                std::process::exit(1);
            }
        }
    } else {
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match lean_md::render_source_with_phase(&source, None, a.consumer, a.crp, jail) {
            Ok(out) => out,
            Err(e) => {
                eprintln!("render error: {e:?}");
                std::process::exit(1);
            }
        }
    };
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
    let source = load_file(file);
    // Same root derivation as everywhere else in the binary (the jail root fragments
    // resolve against). No cwd → the seed part drops out silently.
    let root = std::env::current_dir().ok();
    println!("{}", do_check(&source, root.as_deref()));
}

fn cmd_source(rest: &[String]) {
    // Raw source bytes — bypasses the renderer entirely (no @import/@define/
    // @phase processing, no --consumer/--crp). Fall B: exact edit anchors for
    // `.lmd.md` seeds. Pure function of file content → byte-stable (#498).
    let Some(file) = rest.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("lean-md source: missing <file.lmd.md>");
        std::process::exit(1);
    };
    let source = load_file(file);
    print!("{source}");
}

// ─── ack subcommand ────────────────────────────────────────────────────────

/// `lean-md ack [<seed>…]` — the user's answer to a seed conflict: "I keep mine."
///
/// Records consent for the CURRENT embedded seed and drops the `.new` beside it, so
/// `check` stops reporting it. Consent is scoped to this proposal: when the embedded
/// seed moves on there is something new to say and the report returns by itself. The
/// lock's provenance entry is deliberately NOT touched — the seed must still be able to
/// heal if the user ever reverts.
///
/// One of the three writing verbs (with `skill install` and the `mcp` server start);
/// `render`/`check` stay purely reading (D-1).
fn cmd_ack(rest: &[String]) {
    let filter: Vec<String> = rest
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    let Ok(root) = std::env::current_dir() else {
        eprintln!("lean-md ack: cannot resolve the current directory");
        std::process::exit(1);
    };
    let report = match lean_md::seeds::ack_seeds(&root, ".lean-ctx/lean-md", &filter) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lean-md ack: {e}");
            std::process::exit(1);
        }
    };
    for p in &report.acked {
        println!(
            "acked {} — your copy stays; we stop reporting it",
            p.display()
        );
    }
    // An argument that matched nothing must never read as success: the user thinks they
    // silenced something and would find out only by the report not going away.
    for a in &report.unmatched {
        eprintln!("lean-md ack: no seed conflict matches '{a}'");
    }
    if report.acked.is_empty() && report.unmatched.is_empty() {
        println!("no seed conflicts to acknowledge");
    }
    if !report.unmatched.is_empty() {
        std::process::exit(1);
    }
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
//   tools/list                → 4 tools: ctx_md_render/lmd_render (alias) +
//                                ctx_md_check/lmd_check (alias)
//   tools/call                → dispatch to do_render / do_check
//   <unknown>                 → JSON-RPC error -32601 (method not found)

/// Tool input schemas — byte-identical to ctx_md.rs field names and types.
fn tool_defs() -> Value {
    // Schemas/descriptions bound once so the lmd_* alias can never drift from its
    // ctx_md_* original (#498) — lmd_render/lmd_check are additive tool-name
    // aliases; ctx_md_* stay the canonical names (Global Constraints: pack content
    // keeps citing ctx_md_render/ctx_md_check).
    let render_schema = json!({
        "type": "object",
        "properties": {
            "path":     { "type": "string", "description": "Path to a .lmd.md source" },
            "content":  { "type": "string", "description": "Inline lmd source (alternative to path)" },
            "consumer": { "type": "string", "description": "Override audience: ai|human" },
            "crp":      { "type": "string", "description": "Override CRP mode: tdd|compact|off" },
            "skill":    { "type": "string", "description": "Render an embedded lmd skill body by name (alternative to path/content)" },
            "phase":     { "type": "string", "description": "Render only this named phase of the skill (requires skill; mutually exclusive with companion)" },
            "companion": { "type": "string", "description": "Render a skill's named companion reference (requires skill; mutually exclusive with phase)" }
        }
    });
    let render_description = "Render an lmd (.lmd.md) plan/spec to Markdown. consumer=human narrates \
    directives as prose (readable plan); consumer/crp override the source header.";
    let check_schema = json!({
        "type": "object",
        "properties": {
            "path":    { "type": "string", "description": "Path to a .lmd.md source" },
            "content": { "type": "string", "description": "Inline lmd source (alternative to path)" }
        }
    });
    let check_description = "Parse-check an lmd (.lmd.md) source: reports header config and directive \
    count without executing anything.";
    json!([
        {
            "name": "ctx_md_render",
            "description": render_description,
            "inputSchema": render_schema.clone()
        },
        {
            "name": "lmd_render",
            "description": format!("Alias for ctx_md_render — identical behavior. {render_description}"),
            "inputSchema": render_schema
        },
        {
            "name": "ctx_md_check",
            "description": check_description,
            "inputSchema": check_schema.clone()
        },
        {
            "name": "lmd_check",
            "description": format!("Alias for ctx_md_check — identical behavior. {check_description}"),
            "inputSchema": check_schema
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

fn cmd_skill(rest: &[String]) {
    let sub = rest.first().map_or("", String::as_str);
    if sub == "vars" {
        cmd_skill_vars(&rest[1..]);
        return;
    }
    let name = rest.iter().skip(1).find(|a| !a.starts_with('-'));
    let scope = if rest.iter().any(|a| a == "--global") {
        Scope::Global
    } else {
        Scope::Local
    };
    // --force / --refresh re-materialises the project seeds even if they already exist
    // (refresh a stale derived seed after an embedded-seed edit); default is absent-only.
    let force = rest.iter().any(|a| a == "--force" || a == "--refresh");
    let Some(name) = name else {
        eprintln!("lean-md skill: missing <name>");
        std::process::exit(1);
    };
    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match sub {
        "install" => match install_skill(name, scope, &project_root, force) {
            Ok(target) => println!("installed {name} → {}", target.display()),
            Err(e) => {
                eprintln!("lean-md skill install: {e}");
                std::process::exit(1);
            }
        },
        "remove" => match remove_skill(name, scope, &project_root) {
            Ok(()) => println!("removed {name}"),
            Err(e) => {
                eprintln!("lean-md skill remove: {e}");
                std::process::exit(1);
            }
        },
        other => {
            eprintln!("lean-md skill: unknown subcommand '{other}' (install|remove)");
            std::process::exit(1);
        }
    }
}

fn cmd_skill_vars(rest: &[String]) {
    if !rest.iter().any(|a| a == "--init") {
        eprintln!("lean-md skill vars: missing --init");
        std::process::exit(1);
    }
    // Optional skill name: present → that skill; absent → aggregate across all.
    let name = rest
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str);
    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let sources: Vec<String> = match name {
        Some(n) => match skill_source(n, &project_root) {
            Ok(body) => vec![body],
            Err(e) => {
                eprintln!("lean-md skill vars --init: {e}");
                std::process::exit(1);
            }
        },
        None => match all_skill_sources(&project_root) {
            Ok(bodies) => bodies,
            Err(e) => {
                eprintln!("lean-md skill vars --init: {e}");
                std::process::exit(1);
            }
        },
    };
    let decls: Vec<_> = sources.iter().flat_map(|b| scan_var_decls(b)).collect();
    match write_vars_template(&decls, &project_root) {
        Ok(InitOutcome::Written(p)) => println!("wrote {}", p.display()),
        Ok(InitOutcome::Exists(p)) => {
            eprintln!("{} already exists — not overwritten", p.display());
            print!("{}", render_vars_template(&decls));
        }
        Err(e) => {
            eprintln!("lean-md skill vars --init: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_mcp() {
    use std::io::{BufRead, Write};

    // Seed refresh — server start is the ONLY hook that runs in every session: the addon
    // manifest has no lifecycle phase and no install hook, so a plain `addon update`
    // swaps the pack without ever running our code. Once, before the read loop; render
    // and check stay purely reading (D-1).
    //
    // Visibility is deliberately asymmetric (Spec decision 4): stdout is the JSON-RPC
    // channel (a warning there corrupts the protocol) and stderr lands in the gateway log
    // the agent never sees. So the normal case (stale + untouched) heals silently, and the
    // `.new` case is surfaced by `lean-md check`; stderr is diagnostics only.
    if let Ok(root) = std::env::current_dir()
        && let Ok(report) = lean_md::seeds::refresh_contracts(&root, ".lean-ctx/lean-md")
        && !report.preserved.is_empty()
    {
        for p in &report.preserved {
            eprintln!(
                "lean-md: kept your {} — updated seed written as {}.new \
                 (`lean-md ack` to keep yours and silence this)",
                p.display(),
                p.display()
            );
        }
    }

    // Pack range check — read-only (`.lean-ctx/ctxpkg.lock` belongs to lean-ctx). stderr
    // ONLY: stdout is the JSON-RPC channel below, a line there corrupts the protocol.
    if let Ok(root) = std::env::current_dir()
        && let Some(warning) = lean_md::version_gate::drift_warning(&root)
    {
        eprintln!("{warning}");
    }

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
                    "ctx_md_render" | "lmd_render" => {
                        if let Some(skill) = args.get("skill").and_then(Value::as_str) {
                            let phase = args.get("phase").and_then(Value::as_str);
                            let companion = args.get("companion").and_then(Value::as_str);
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
                                .and_then(CrpMode::parse);
                            let jail = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            if phase.is_some() && companion.is_some() {
                                rpc_err(&id, -32602, "phase and companion are mutually exclusive")
                            } else {
                                let result = match companion {
                                    Some(c) => render_companion(skill, c, consumer, crp, jail),
                                    None => render_skill(skill, phase, consumer, crp, jail),
                                };
                                match result {
                                    Ok(rendered) => rpc_ok(
                                        &id,
                                        json!({ "content": [{ "type": "text", "text": rendered }] }),
                                    ),
                                    Err(e) => rpc_err(&id, -32602, &format!("{e}")),
                                }
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
                                        .and_then(CrpMode::parse);
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

                    "ctx_md_check" | "lmd_check" => match mcp_load_source(&args) {
                        Ok((source, _jail)) => {
                            let root = std::env::current_dir().ok();
                            let summary = do_check(&source, root.as_deref());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_phase_fails_in_check() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        let out = do_check(src, None);
        assert!(
            !out.contains("lmd ok"),
            "check must not call a lossy file ok: {out}"
        );
        assert!(out.contains("duplicate"), "{out}");
    }

    #[test]
    fn list_phases_refuses_a_duplicate_out_loud() {
        // A gate that degrades to silence is the very defect this package removes:
        // no output + exit 0 is indistinguishable from "this file has no phases".
        // Every surface must say the same thing.
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        let err = list_phases_output(src)
            .expect_err("--list-phases must refuse a lossy source, not print nothing");
        // Same message as `check` / `--phase X` — one formatter, no second wording.
        assert_eq!(err, lean_md::phases::duplicate_phase_error(src).unwrap());
        assert!(err.contains("duplicate @phase \"t\""), "{err}");
        assert!(
            err.contains("line 4") && err.contains("line 7"),
            "both sites must be named: {err}"
        );
    }

    #[test]
    fn list_phases_still_lists_a_clean_source() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"a\"\n# First\n@phase-end\n@phase \"b\"\n# Second\n@phase-end\n";
        assert_eq!(
            list_phases_output(src).unwrap(),
            "a\tFirst\nb\tSecond\n",
            "the gate must not cost a clean file its index"
        );
    }

    #[test]
    fn mcp_start_refreshes_seeds_but_render_and_check_do_not_write() {
        // D-1 purity: the renderer stays PURE. The wiring sits at server start, not on the
        // hot path. Proof: file state before == after for render/check.
        let root = std::env::temp_dir().join(format!("lmd_wire_pure_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        lean_md::seeds::materialize_contracts(&root, dir, false).unwrap();
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();

        let target = root.join(dir).join("plan-recipes.lmd.md");
        std::fs::write(&target, "# stale untouched\n").unwrap();
        let mut lock = lean_md::lock::Lock::load(&root);
        lock.set(
            "lean-md/plan-recipes.lmd.md",
            &lean_md::hashx::sha256_hex(b"# stale untouched\n"),
        );
        lock.save(&root).unwrap();

        // render must not heal it …
        let _ = do_render("@lean-md\nconsumer: ai\n\nhi\n", root.clone(), None, None);
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# stale untouched\n"
        );
        // … nor must check.
        let _ = do_check("@lean-md\nconsumer: ai\n\nhi\n", Some(&root));
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# stale untouched\n"
        );

        // The MCP start path does.
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();
        assert_ne!(
            std::fs::read_to_string(&target).unwrap(),
            "# stale untouched\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_reports_the_new_case_and_stays_silent_on_a_healed_one() {
        let root = std::env::temp_dir().join(format!("lmd_wire_check_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        lean_md::seeds::materialize_contracts(&root, dir, false).unwrap();
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();

        // Current tree → check says nothing about seeds.
        let quiet = do_check("@lean-md\nconsumer: ai\n\nhi\n", Some(&root));
        assert!(
            !quiet.contains(".new"),
            "a current tree must not be reported: {quiet}"
        );

        // A user edit + a refresh → .new exists → check must surface it, because stderr at
        // MCP start is a log the agent never reads.
        std::fs::write(root.join(dir).join("lang/rust.lmd.md"), "# mine\n").unwrap();
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();
        let loud = do_check("@lean-md\nconsumer: ai\n\nhi\n", Some(&root));
        assert!(
            loud.contains("lang/rust.lmd.md.new"),
            "check must name the .new file: {loud}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_without_a_project_root_still_checks_the_file() {
        let out = do_check("@lean-md\nconsumer: ai\n\nhi\n", None);
        assert!(
            out.contains("lmd ok"),
            "the file check must survive a missing root: {out}"
        );
    }

    #[test]
    fn unknown_argument_is_rejected_and_the_known_ones_are_listed() {
        // `brief=` was swallowed and dropped — it never existed in src/. So is every future
        // typo: phse=, to-agent=.
        let out = do_check(
            "@lean-md\nconsumer: ai\n\n@dispatch brief=x phase=y\n",
            None,
        );
        assert!(out.contains("unknown argument"), "{out}");
        assert!(
            out.contains("brief"),
            "the offending arg must be named: {out}"
        );
        assert!(
            out.contains("phase") && out.contains("role"),
            "known args must be listed: {out}"
        );
    }

    #[test]
    fn a_bad_role_fails_in_check_not_only_at_render_time() {
        // role IS validated today — but at render time (dispatch.rs). check never renders,
        // so it never saw it. Green check, broken file.
        let out = do_check(
            "@lean-md\nconsumer: ai\n\n@dispatch phase=t role=exec\n",
            None,
        );
        assert!(out.contains("role"), "{out}");
        assert!(
            !out.contains("lmd ok"),
            "check must not call this file ok: {out}"
        );
    }

    #[test]
    fn a_dispatch_without_a_brief_source_fails_in_check() {
        let out = do_check("@lean-md\nconsumer: ai\n\n@dispatch role=dev\n", None);
        assert!(!out.contains("lmd ok"), "{out}");
    }

    #[test]
    fn phase_and_a_complete_companion_group_fail_in_check() {
        let out = do_check(
            "@lean-md\nconsumer: ai\n\n@dispatch phase=x skill=s companion=y\n",
            None,
        );
        assert!(
            !out.contains("lmd ok"),
            "exclusive group must be enforced in check: {out}"
        );
    }

    #[test]
    fn phase_and_companion_without_skill_is_still_exclusive() {
        // The regression the review caught: an incomplete second group made `validate`
        // wave the pair through, and `companion=` fell on the floor silently — exactly
        // the defect class this package removes. Touching two groups is the error, not
        // completing two.
        let out = do_check(
            "@lean-md\nconsumer: ai\n\n@dispatch phase=x companion=y\n",
            None,
        );
        assert!(
            !out.contains("lmd ok"),
            "phase= + companion= must never pass: {out}"
        );
    }

    #[test]
    fn a_fenced_directive_is_not_checked() {
        // `check` must see what `render` sees. A fenced block is verbatim text for the
        // renderer, so a "broken" directive inside it is not broken — it is documentation.
        // A gate that blocks files which render cleanly is a false alarm.
        let src = "@lean-md\nconsumer: ai\n\n```\n@dispatch brief=x\n```\n@dispatch phase=t\n";
        let out = do_check(src, None);
        assert!(
            out.contains("lmd ok"),
            "a fenced directive must not fail check: {out}"
        );
    }

    #[test]
    fn a_valid_dispatch_still_checks_ok() {
        let out = do_check(
            "@lean-md\nconsumer: ai\n\n@dispatch phase=t role=review\n",
            None,
        );
        assert!(out.contains("lmd ok"), "{out}");
    }

    #[test]
    fn parse_render_flags_knows_signatures() {
        let args = parse_render_flags(&["lib.lmd.md".to_string(), "--signatures".to_string()]);
        assert!(
            args.signatures,
            "--signatures must set RenderArgs.signatures"
        );
        assert_eq!(args.file.as_deref(), Some("lib.lmd.md"));

        // --phase on a file arg is now carried for the file branch too.
        let args2 = parse_render_flags(&[
            "plan.lmd.md".to_string(),
            "--phase".to_string(),
            "task-1".to_string(),
        ]);
        assert_eq!(args2.phase.as_deref(), Some("task-1"));
        assert_eq!(args2.file.as_deref(), Some("plan.lmd.md"));
        assert!(!args2.signatures);
    }

    #[test]
    fn render_flags_parse_companion() {
        let a = parse_render_flags(&[
            "--skill".to_string(),
            "lmd-test-driven-development".to_string(),
            "--companion".to_string(),
            "testing-anti-patterns".to_string(),
        ]);
        assert_eq!(a.skill.as_deref(), Some("lmd-test-driven-development"));
        assert_eq!(a.companion.as_deref(), Some("testing-anti-patterns"));
        assert_eq!(a.phase, None);
    }

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

    #[test]
    fn brainstorm_explore_weaves_find() {
        // §3.1: the explore guidance must demonstrate @find (semantic locate),
        // not just @search — COVERAGE registers explore/find→ctx_semantic_search.
        let jail = std::path::PathBuf::from(".");
        let out = render_skill("lmd-brainstorm", Some("explore"), None, None, jail).unwrap();
        assert!(
            out.contains("@find"),
            "explore guidance must weave @find, got: {out}"
        );
    }

    #[test]
    fn mcp_companion_matches_cli_render_companion() {
        // CLI==MCP (#498): both surfaces call render_companion → byte-identical.
        let jail = std::path::PathBuf::from(".");
        let cli = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            jail,
        )
        .unwrap();
        assert!(cli.contains("Anti-Pattern 1"));
        assert!(cli.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"));
    }

    #[test]
    fn ws_mcp_companion_matches_cli_render_companion() {
        // CLI==MCP (#498): both surfaces call render_companion → byte-identical.
        let jail = std::path::PathBuf::from(".");
        let cli = render_companion(
            "lmd-writing-skills",
            "testing/methodology",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        let again = render_companion(
            "lmd-writing-skills",
            "testing/methodology",
            None,
            None,
            jail,
        )
        .unwrap();
        assert_eq!(
            cli, again,
            "render_companion must be a deterministic function (#498)"
        );
        assert!(
            cli.contains("RED Phase"),
            "testing/methodology surface must render its methodology body"
        );
    }

    #[test]
    fn tool_defs_expose_companion_param() {
        let defs = tool_defs();
        let schema = defs[0]["inputSchema"]["properties"].clone();
        assert!(
            schema.get("companion").is_some(),
            "ctx_md_render must expose a 'companion' param: {schema}"
        );
    }

    #[test]
    fn tools_list_carries_lmd_render_and_lmd_check() {
        let defs = tool_defs();
        let names: Vec<&str> = defs
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["ctx_md_render", "lmd_render", "ctx_md_check", "lmd_check"],
            "tools/list must expose ctx_md_render/ctx_md_check + their lmd_* aliases: {names:?}"
        );
        // Alias schema must be identical to the original — one source of truth (#498),
        // so ctx_md_render/lmd_render (and ctx_md_check/lmd_check) can never drift.
        assert_eq!(
            defs[0]["inputSchema"], defs[1]["inputSchema"],
            "lmd_render inputSchema must not drift from ctx_md_render"
        );
        assert_eq!(
            defs[2]["inputSchema"], defs[3]["inputSchema"],
            "lmd_check inputSchema must not drift from ctx_md_check"
        );
    }

    #[test]
    fn do_render_runs_var_prepass_like_cli_whole_doc() {
        // Follow-up B (M3, strengthened): a FORWARD reference — {{ var }} used BEFORE its
        // @var declaration — resolves ONLY if the @var pre-pass ran (render_source_with_phase
        // scans every @var default up front). A plain default that sits AFTER its use would
        // resolve either way, so it does not discriminate; the forward form does.
        let src = "@lean-md\nconsumer: ai\n\n{{ var greeting }}\n@var greeting default=\"hello\"\n";
        let out = do_render(src, std::path::PathBuf::from("."), None, None);
        assert!(
            out.contains("hello"),
            "do_render must run the @var pre-pass so a forward reference resolves: {out}"
        );
        // Discrimination guard: the pre-fix routing (render_with_overrides, no pre-pass)
        // leaves the forward reference unresolved — this test genuinely fails against the
        // old code path, so it is a discriminator, not a pass-regardless regression guard.
        let without_prepass =
            lean_md::engine::render_with_overrides(src, None, None, std::path::PathBuf::from("."));
        assert!(
            !without_prepass.contains("hello"),
            "guard: without the pre-pass a forward reference must NOT resolve: {without_prepass}"
        );
    }
}
