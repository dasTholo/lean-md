//! Vendored from lean-ctx core::tdd_schema (tdd_schema_value only).

use serde_json::{Value, json};

pub fn tdd_schema_value() -> Value {
    json!({
        "schema_version": 1,
        "format": "lean-ctx-tdd",
        "crp": {
            "modes": [
                {
                    "name": "off",
                    "description": "No CRP transformation."
                },
                {
                    "name": "compact",
                    "description": "Compact prose; prefer bullet points and short lines."
                },
                {
                    "name": "tdd",
                    "description": "Token Dense Dialect: max information density, minimal narration."
                }
            ],
            "output_rules": [
                "Prefer structured bullets over paragraphs.",
                "Avoid repeating previously shown context.",
                "Show diffs instead of full files when possible.",
                "For code reads, prefer map/signatures/task over full."
            ]
        },
        "ctx_read": {
            "read_modes": [
                {"name":"auto","description":"Predict best mode (predictor + adaptive policy)."},
                {"name":"full","description":"Full file content (cached)."},
                {"name":"map","description":"Deps + exports + key API signatures (TOON)."},
                {"name":"signatures","description":"API surface only."},
                {"name":"diff","description":"Changed lines since last read."},
                {"name":"aggressive","description":"Whitespace/comment stripping with safeguards."},
                {"name":"entropy","description":"Entropy/Jaccard-based extraction."},
                {"name":"task","description":"Task-aware compression (IB filter + graph context)."},
                {"name":"reference","description":"Header-only reference (lines/tokens), no content."},
                {"name":"lines:N-M","description":"Line-range extraction with line numbers."}
            ],
            "toon_header": {
                "deps": "  deps: a, b, c",
                "exports": "  exports: x, y",
                "api": "  API:\\n    <signature>"
            },
            "file_ref_format": "F<idx>=<short-path> <line-count>L",
            "compressed_hint": "[lean-ctx: compact view — nothing lost, full source on request]"
        },
        "stability": {
            "determinism": [
                "Sorted keys for manifests/exports.",
                "Stable ordering for ledgers and reports."
            ],
            "local_first": [
                "All files stored under LEAN_CTX_DATA_DIR by default.",
                "No raw prompts stored."
            ]
        }
    })
}
