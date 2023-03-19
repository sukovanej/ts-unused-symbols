use std::{collections::HashSet, hash::Hash, path::PathBuf};

use swc_ecma_ast::Ident;

#[derive(Debug, Clone, Default)]
pub struct ModuleSymbols<P> {
    pub exports: HashSet<Export<P>>,
    pub imports: HashSet<ImportedSymbol<P>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Export<P> {
    Symbol(String),
    Reexport(Reexport<P>),
    AllFrom(P),
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reexport<P> {
    pub from: P,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportedSymbol<P> {
    pub symbols: Vec<Import>,
    pub from: P,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Import {
    Named(String),
    Default,
    Namespace,
}

impl ModuleSymbols<String> {
    pub fn new_imported_symbol(imported_symbol: ImportedSymbol<String>) -> Self {
        Self {
            imports: HashSet::from([imported_symbol]),
            ..Default::default()
        }
    }

    pub fn new_exported_symbol(symbol: Ident) -> Self {
        Self::new_exported_symbol_str(symbol.sym.to_string())
    }

    pub fn new_exported_symbol_str(symbol: String) -> Self {
        Self {
            exports: HashSet::from([Export::Symbol(symbol)]),
            ..Default::default()
        }
    }

    pub fn new_all_export(from: String) -> Self {
        Self {
            exports: HashSet::from([Export::AllFrom(from)]),
            ..Default::default()
        }
    }

    pub fn merge(self, analyzed_module: Self) -> Self {
        let mut exports = self.exports;
        exports.extend(analyzed_module.exports);

        let mut imports = self.imports;
        imports.extend(analyzed_module.imports);

        Self { exports, imports }
    }
}

pub fn merge_iter<Iter: IntoIterator<Item = ModuleSymbols<String>>>(
    analyzed_modules: Iter,
) -> ModuleSymbols<String> {
    let mut analyzed_module = ModuleSymbols::default();

    for other_analyzed_module in analyzed_modules {
        analyzed_module = analyzed_module.merge(other_analyzed_module);
    }

    analyzed_module
}
