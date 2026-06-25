//! `lean-md` — standalone CLI: render/check `.lmd.md` sources.
//! Mirrors the `lean-ctx md <render|check>` subcommand against the decoupled
//! `lean_md` library crate (no lean-ctx engine linkage). Code-intel directives
//! degrade to the `BACKEND_REQUIRED` envelope unless `lean-ctx` is on PATH
//! (the `CliBackend` default shells out to it).

use lean_md::crp_proto::CrpMode;
use lean_md::engine::render_with_overrides;
use lean_md::header::{Consumer, parse_header};

#[derive(Debug, Default, PartialEq)]
struct RenderArgs {
    file: Option<String>,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    out: Option<String>,
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
            _ if !arg.starts_with('-') && a.file.is_none() => a.file = Some(arg.to_string()),
            _ => {}
        }
        i += 1;
    }
    a
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let action = args.first().map_or("help", String::as_str);
    match action {
        "render" => cmd_render(&args[1..]),
        "check" => cmd_check(&args[1..]),
        _ => {
            eprintln!(
                "Usage: lean-md <render|check> <file.lmd.md> [--consumer=human] [--crp=off] [-o out.md]"
            );
            std::process::exit(1);
        }
    }
}

fn cmd_render(rest: &[String]) {
    let a = parse_render_flags(rest);
    let Some(file) = a.file else {
        eprintln!("lean-md render: missing <file.lmd.md>");
        std::process::exit(1);
    };
    let source = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lean-md render: read {file}: {e}");
            std::process::exit(1);
        }
    };
    let jail = std::path::Path::new(&file).parent().map_or_else(
        || std::path::PathBuf::from("."),
        std::path::Path::to_path_buf,
    );
    let rendered = render_with_overrides(&source, a.consumer, a.crp, jail);
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

fn cmd_check(rest: &[String]) {
    let Some(file) = rest.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("lean-md check: missing <file.lmd.md>");
        std::process::exit(1);
    };
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lean-md check: read {file}: {e}");
            std::process::exit(1);
        }
    };
    let (header, body) = parse_header(&source);
    let directives = body
        .lines()
        .filter(|l| l.trim_start().starts_with('@'))
        .count();
    println!(
        "lmd ok — consumer={:?}, crp={:?}, directives={}",
        header.consumer, header.crp, directives
    );
}
