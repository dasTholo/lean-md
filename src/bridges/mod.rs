//! Directive bridges: each Router directive is a thin bridge into an existing
//! lean-ctx core API (spec §4.2). `execute` takes `&Rc<EngineContext>` so a
//! bridge can re-enter the engine (e.g. `@include` renders its fragment).

pub mod addressing;
pub mod count;
pub mod date;
pub mod edit;
pub mod env;
pub mod graph;
pub mod include;
pub mod list;
pub mod query;
pub mod read;
pub mod refactor;
pub mod search;
pub mod symbol;

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
}

pub trait DirectiveBridge {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &Rc<EngineContext>, args: &DirectiveArgs)
               -> Result<String, BridgeError>;
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
        self.map.get(name).map(|b| b.as_ref())
    }
}

pub fn default_registry() -> BridgeRegistry {
    let mut reg = BridgeRegistry::new();
    reg.register(Box::new(count::CountBridge));
    reg.register(Box::new(date::DateBridge));
    reg.register(Box::new(edit::EditBridge));
    reg.register(Box::new(env::EnvBridge));
    reg.register(Box::new(graph::GraphBridge));
    reg.register(Box::new(read::ReadBridge));
    reg.register(Box::new(include::IncludeBridge));
    reg.register(Box::new(search::SearchBridge));
    reg.register(Box::new(list::ListBridge));
    reg.register(Box::new(query::QueryBridge));
    reg.register(Box::new(symbol::SymbolBridge));
    reg.register(Box::new(refactor::RefactorBridge));
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
    fn default_registry_has_all_core_bridges() {
        let reg = default_registry();
        for name in [
            "read", "include", "search", "list", "env", "date", "count", "query", "graph", "edit",
            "symbol", "refactor",
        ] {
            assert!(reg.get(name).is_some(), "missing bridge: {name}");
        }
    }
}
