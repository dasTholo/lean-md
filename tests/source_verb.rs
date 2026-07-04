//! `lean-md source <file.lmd.md>` — raw file bytes, NO rendering (Fall B: edit
//! anchors for `.lmd.md` seeds). Contrast: `render` of the same file consumes
//! the macros. Proves the raw-source path bypasses the renderer (spec §Design.1).
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_lean-md");

/// A fixture whose macros the renderer WOULD consume: a failing `@import`
/// (NotFound cascade) plus a local `@define`/`@call` pair. `source` must return
/// every byte verbatim; `render` must not.
const FIXTURE: &str = "# Fixture\n@import ./does-not-exist /\n@define greet(name) = Hello name\n@call greet(\"world\")\ntail line\n";

fn write_fixture() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("lmd_source_verb_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fixture.lmd.md");
    std::fs::write(&path, FIXTURE).unwrap();
    path
}

#[test]
fn source_emits_raw_bytes_verbatim() {
    let path = write_fixture();
    let out = Command::new(BIN)
        .arg("source")
        .arg(&path)
        .output()
        .expect("run lean-md source");
    assert!(out.status.success(), "source must exit 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Byte-identical to the on-disk source — no rendering, no NotFound comment.
    assert_eq!(stdout, FIXTURE, "source must be byte-identical to the file");
    assert!(
        stdout.contains("@define greet(name)"),
        "raw source must keep the @define directive verbatim"
    );
    assert!(
        !stdout.contains("NotFound"),
        "source must not render the @import (no NotFound comment)"
    );
}

#[test]
fn render_consumes_macros_source_does_not() {
    let path = write_fixture();
    let source = Command::new(BIN)
        .arg("source")
        .arg(&path)
        .output()
        .expect("run lean-md source");
    let rendered = Command::new(BIN)
        .arg("render")
        .arg(&path)
        .output()
        .expect("run lean-md render");
    let source = String::from_utf8(source.stdout).unwrap();
    let rendered = String::from_utf8(rendered.stdout).unwrap();
    // Counter-assert: the renderer consumes the @define macro; raw source keeps it.
    assert_ne!(rendered, source, "render must differ from raw source");
    assert!(
        !rendered.contains("@define greet(name)"),
        "render must consume the @define macro"
    );
}

#[test]
fn source_missing_arg_fails() {
    let out = Command::new(BIN)
        .arg("source")
        .output()
        .expect("run lean-md source");
    assert!(!out.status.success(), "missing <file> must exit non-zero");
}
