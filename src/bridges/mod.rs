//! Directive bridges: each Router directive is a thin bridge into an existing
//! lean-ctx core API (spec §4.2). `execute` takes `&Rc<EngineContext>` so a
//! bridge can re-enter the engine (e.g. `@include` renders its fragment).

pub mod addressing;
pub mod architecture;
pub mod call;
pub mod checkpoint;
pub mod compress;
pub mod count;
pub mod date;
pub mod dispatch;
pub mod edit;
pub mod env;
pub mod find;
pub mod graph;
pub mod handoff;
pub mod impact;
pub mod include;
pub mod inspect;
pub mod list;
pub mod outline;
pub mod query;
pub mod read;
pub mod recall;
pub mod refactor;
pub mod reformat;
pub mod remember;
pub mod render;
pub mod repomap;
pub mod review;
pub mod routes;
pub mod search;
pub mod smells;
pub mod symbol;
pub mod var;

use std::collections::HashMap;
use std::rc::Rc;

use super::args::DirectiveArgs;
use super::engine::EngineContext;

#[derive(Debug)]
pub enum BridgeError {
    MissingArg(&'static str),
    Resolve(String),
    Io(String),
    DepthExceeded,
    /// `@query` invoked without `@lean-md shell=allow` (Spec §7 consumer gate).
    ShellDenied,
    /// `@query` command rejected by an inherited lean-ctx shell defense
    /// (strict-mode `$()`/backtick block or shell allowlist).
    ShellRejected(String),
    /// A real outbound `ctx.backend.call(...)` failure (Spawn/NonZero/Io) — e.g.
    /// `lean-ctx` unreachable or a PathJail reject (`NonZero{stderr}`). Distinct
    /// from a tool-owned `ERROR:` envelope (tool exit 0): this variant propagates
    /// as `Err` so a failing code-intel call inside a `@phase` aborts the phase
    /// (I2). Display is a pure function of the `BackendError` content (#498).
    Backend(crate::backend::BackendError),
}

impl From<crate::backend::BackendError> for BridgeError {
    fn from(e: crate::backend::BackendError) -> Self {
        BridgeError::Backend(e)
    }
}

impl std::fmt::Display for BridgeError {
    /// Readable, byte-stable (#498) rendering — used in the `PHASE_ABORTED`
    /// envelope (`phases.rs`). `Backend(e)` defers to `BackendError`'s own
    /// `Display` (e.g. `backend exit 1: <stderr>`), prefixed `BACKEND_REQUIRED:`
    /// so the historic envelope marker survives. No timestamps/counters.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::MissingArg(a) => write!(f, "missing arg: {a}"),
            BridgeError::Resolve(m) => write!(f, "resolve: {m}"),
            BridgeError::Io(m) => write!(f, "io: {m}"),
            BridgeError::DepthExceeded => f.write_str("include depth exceeded"),
            BridgeError::ShellDenied => f.write_str("shell denied"),
            BridgeError::ShellRejected(m) => write!(f, "shell rejected: {m}"),
            BridgeError::Backend(e) => write!(f, "BACKEND_REQUIRED: {e}"),
        }
    }
}

impl std::error::Error for BridgeError {}

pub trait DirectiveBridge {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &Rc<EngineContext>, args: &DirectiveArgs)
    -> Result<String, BridgeError>;
    /// Whether this bridge consumes an upstream pipe's output (spec §5).
    /// Default `false`: piping into it is a visible error.
    fn accepts_pipe(&self) -> bool {
        false
    }
}

/// Name-keyed registry of directive bridges.
#[derive(Default)]
pub struct BridgeRegistry {
    map: HashMap<&'static str, Box<dyn DirectiveBridge>>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn register(&mut self, bridge: Box<dyn DirectiveBridge>) {
        self.map.insert(bridge.name(), bridge);
    }
    pub fn get(&self, name: &str) -> Option<&dyn DirectiveBridge> {
        self.map.get(name).map(std::convert::AsRef::as_ref)
    }
}

pub fn default_registry() -> BridgeRegistry {
    let mut reg = BridgeRegistry::new();
    reg.register(Box::new(architecture::ArchitectureBridge));
    reg.register(Box::new(call::CallBridge));
    reg.register(Box::new(checkpoint::CheckpointBridge));
    reg.register(Box::new(compress::CompressBridge));
    reg.register(Box::new(count::CountBridge));
    reg.register(Box::new(date::DateBridge));
    reg.register(Box::new(dispatch::DispatchBridge));
    reg.register(Box::new(edit::EditBridge));
    reg.register(Box::new(env::EnvBridge));
    reg.register(Box::new(graph::GraphBridge));
    reg.register(Box::new(handoff::HandoffBridge));
    reg.register(Box::new(find::FindBridge));
    reg.register(Box::new(impact::ImpactBridge));
    reg.register(Box::new(repomap::RepomapBridge));
    reg.register(Box::new(read::ReadBridge));
    reg.register(Box::new(include::IncludeBridge));
    reg.register(Box::new(search::SearchBridge));
    reg.register(Box::new(list::ListBridge));
    reg.register(Box::new(outline::OutlineBridge));
    reg.register(Box::new(query::QueryBridge));
    reg.register(Box::new(symbol::SymbolBridge));
    reg.register(Box::new(refactor::RefactorBridge));
    reg.register(Box::new(reformat::ReformatBridge));
    reg.register(Box::new(remember::RememberBridge));
    reg.register(Box::new(recall::RecallBridge));
    reg.register(Box::new(inspect::InspectBridge));
    reg.register(Box::new(review::ReviewBridge));
    reg.register(Box::new(routes::RoutesBridge));
    reg.register(Box::new(smells::SmellsBridge));
    reg.register(Box::new(render::RenderBridge));
    reg.register(Box::new(var::VarBridge));
    reg
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_registers_and_gets() {
        let mut reg = BridgeRegistry::new();
        reg.register(Box::new(read::ReadBridge));
        assert!(reg.get("read").is_some());
        assert!(reg.get("nope").is_none());
    }
    #[test]
    fn checkpoint_and_compress_bridges_registered() {
        let reg = default_registry();
        assert!(reg.get("checkpoint").is_some(), "checkpoint bridge missing");
        assert!(reg.get("compress").is_some(), "compress bridge missing");
    }

    #[test]
    fn default_registry_has_all_core_bridges() {
        let reg = default_registry();
        for name in [
            "read",
            "include",
            "search",
            "list",
            "env",
            "date",
            "dispatch",
            "count",
            "query",
            "graph",
            "handoff",
            "edit",
            "symbol",
            "refactor",
            "reformat",
            "inspect",
            "find",
            "repomap",
            "impact",
            "architecture",
            "outline",
            "call",
            "review",
            "routes",
            "smells",
            "remember",
            "recall",
            "render",
        ] {
            assert!(reg.get(name).is_some(), "missing bridge: {name}");
        }
    }
}
