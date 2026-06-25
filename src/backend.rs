//! Outbound code-intel boundary (spec §3). Every directive that needs a
//! ctx_* tool calls `backend.call(tool, args)`; the result is the tool's raw
//! text output (byte-stable per #498). CliBackend is the default; McpBackend
//! is opt-in behind the `mcp` feature.

use std::process::Command;

#[derive(Debug)]
pub enum BackendError {
    Spawn(String),
    NonZero { code: i32, stderr: String },
    Io(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::Spawn(m) => write!(f, "backend spawn failed: {m}"),
            BackendError::NonZero { code, stderr } => {
                write!(f, "backend exit {code}: {stderr}")
            }
            BackendError::Io(m) => write!(f, "backend io: {m}"),
        }
    }
}

impl std::error::Error for BackendError {}

pub trait CodeIntelBackend {
    /// Call a lean-ctx code-intel tool over the wire; return its raw text.
    fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError>;
}

/// Default backend: `lean-ctx call <tool> --project-root <root> --json '<args>'`.
/// Stateless — one short-lived process per call (spec §3.1).
pub struct CliBackend {
    pub project_root: String,
}

impl CodeIntelBackend for CliBackend {
    fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError> {
        let json = serde_json::to_string(&args).map_err(|e| BackendError::Io(e.to_string()))?;
        let out = Command::new("lean-ctx")
            .arg("call")
            .arg(tool)
            .arg("--project-root")
            .arg(&self.project_root)
            .arg("--json")
            .arg(&json)
            .output()
            .map_err(|e| BackendError::Spawn(e.to_string()))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(BackendError::NonZero {
                code: out.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
    }
}

/// Backend selection (spec §3.2): CLI default; MCP only when explicitly
/// requested via `LEAN_MD_BACKEND=mcp` + `LEAN_MD_MCP_ENDPOINT`.
pub fn default_backend(project_root: &str) -> Box<dyn CodeIntelBackend> {
    #[cfg(feature = "mcp")]
    if std::env::var("LEAN_MD_BACKEND").as_deref() == Ok("mcp") {
        if let Ok(endpoint) = std::env::var("LEAN_MD_MCP_ENDPOINT") {
            return Box::new(mcp::McpBackend::new(endpoint, project_root.to_string()));
        }
    }
    Box::new(CliBackend {
        project_root: project_root.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_backend_calls_ctx_tree() {
        // Requires `lean-ctx` in PATH. ctx_tree on a temp dir returns the marker.
        let dir = std::env::temp_dir().join("lean_md_cli_be");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("MARKER.txt"), b"x").unwrap();
        let be = CliBackend {
            project_root: dir.to_string_lossy().into_owned(),
        };
        let out = be.call("ctx_tree", serde_json::json!({"path": "."})).unwrap();
        assert!(out.contains("MARKER.txt"), "got: {out}");
    }

    #[test]
    fn cli_backend_unknown_tool_errs_or_envelopes() {
        let dir = std::env::temp_dir();
        let be = CliBackend {
            project_root: dir.to_string_lossy().into_owned(),
        };
        let res = be.call("definitely_not_a_tool", serde_json::json!({}));
        // CLI exits non-zero on unknown tool → NonZero error.
        assert!(
            matches!(res, Err(BackendError::NonZero { .. })),
            "got: {res:?}"
        );
    }
}
