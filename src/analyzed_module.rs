use std::path::PathBuf;

use crate::module_symbols::ModuleSymbols;

#[derive(Clone)]
pub struct AnalyzedModule {
    pub path: PathBuf,
    pub symbols: ModuleSymbols,
}

impl AnalyzedModule {
    pub fn new(path: PathBuf, symbols: ModuleSymbols) -> Self {
        Self { path, symbols }
    }

    pub fn debug(&self) -> String {
        let path = self.path.to_str().unwrap();
        format!("File: {}\n{}", path, self.symbols.debug())
    }
}
