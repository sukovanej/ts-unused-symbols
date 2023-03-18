use std::fmt::Debug;

use crate::module_symbols::ModuleSymbols;

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
