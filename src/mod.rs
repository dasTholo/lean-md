//! `lmd` — native lean-ctx Live-Markdown engine.
//! Phase 0 contains only the executable R/H/E necessity-audit (`audit`).
//! No parser or bridge logic exists yet; that scope is decided by the
//! Phase-0 gate (see docs/lean-md/decisions/2026-05-31-phase-0-gate-outcome.md).

pub mod args;
pub mod audit;
pub mod fragments;
pub mod header;
