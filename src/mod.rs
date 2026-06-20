//! `lmd` — native lean-ctx Live-Markdown engine.
//! Phase 1: `@lean-md` header pre-scan + a rushdown extension (block/inline
//! directive parsers → AST nodes → render-time bridge dispatch) wiring
//! `@read` (R-router → ctx_read) and `@include` (built-in-first fragments).
//! See docs/lean-md/plans/2026-06-01-lmd-phase-1.md.

pub mod args;
pub mod audit;
pub mod bridges;
pub mod engine;
pub mod fragments;
pub mod header;
pub mod macros;
pub mod node;
pub mod parser;
pub mod render;
