//! lean-md — standalone macro/directive markdown renderer.
//! Render core (rushdown/evalexpr) is in-process; code-intel is outbound
//! via `backend::CodeIntelBackend` (CLI default, MCP opt-in).

pub mod args;
pub mod audit;
pub mod auto_findings;
pub mod availability;
pub mod backend;
pub mod bridges;
pub mod crp;
pub mod crp_proto;
pub mod crp_schema;
pub mod engine;
pub mod fragments;
mod gloss;
pub mod header;
pub mod macros;
pub mod node;
pub mod parser;
pub mod pathx;
pub mod phases;
pub mod render;
pub mod seeds;
pub mod signatures;
pub mod skill_install;
pub mod skills;
#[cfg(test)]
mod test_env;

pub use engine::{EngineContext, render, render_body, render_with_overrides};
pub use fragments::FragmentRegistry;
