use std::fmt::Debug;

use crate::module_symbols::{Export, ModuleSymbols};

#[derive(Clone, Debug)]
pub struct AnalyzedModule<P> {
    pub path: P,
    pub symbols: ModuleSymbols<P>,
}

impl AnalyzedModule<String> {
    pub fn new(path: String, symbols: ModuleSymbols<String>) -> Self {
        Self { path, symbols }
    }
}

impl<P> AnalyzedModule<P> {
    pub fn exports_symbol(&self, symbol: &str) -> bool {
        for s in &self.symbols.exports {
            if let Export::Symbol(s) = s {
                if s == symbol {
                    return true;
                }
            }
        }

        false
    }
}
