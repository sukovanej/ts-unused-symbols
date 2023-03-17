use std::collections::HashSet;

use swc_ecma_ast::Ident;

#[derive(Debug, Clone, Default)]
pub struct ModuleSymbols {
    pub defined_symbols: HashSet<Ident>,
    pub used_symbols: HashSet<Ident>,
    pub exported_symbols: HashSet<Ident>,
    pub imported_symbols: HashSet<ImportedSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportedSymbol {
    pub symbols: Vec<Ident>,
    pub from: String,
}

impl ImportedSymbol {
    fn debug(&self) -> String {
        let symbols: Vec<String> = self
            .symbols
            .clone()
            .into_iter()
            .map(|i| i.sym.to_string())
            .collect();

        format!("Imported {} from {}", symbols.join(", "), self.from)
    }
}

impl ModuleSymbols {
    pub fn debug(&self) -> String {
        let defined_symbols: Vec<String> = self
            .defined_symbols
            .clone()
            .into_iter()
            .map(|i| i.sym.to_string())
            .collect();

        let used_symbols: Vec<String> = self
            .used_symbols
            .clone()
            .into_iter()
            .map(|i| i.sym.to_string())
            .collect();

        let exported_symbols: Vec<String> = self
            .exported_symbols
            .clone()
            .into_iter()
            .map(|i| i.sym.to_string())
            .collect();

        let imported_symbols: Vec<String> = self
            .imported_symbols
            .clone()
            .into_iter()
            .map(|i| i.debug())
            .collect();

        format!(
            "Defined symbols:\n{:?}\nUsed symbols:\n{:?}\nExported symbols\n{:?}\nImported symbols\n{:?}",
            defined_symbols, used_symbols, exported_symbols, imported_symbols
        )
    }

    pub fn new_defined_symbol(defined_symbol: Ident) -> ModuleSymbols {
        Self {
            defined_symbols: HashSet::from([defined_symbol]),
            ..Default::default()
        }
    }

    pub fn new_imported_symbol(imported_symbol: ImportedSymbol) -> ModuleSymbols {
        Self {
            imported_symbols: HashSet::from([imported_symbol]),
            ..Default::default()
        }
    }

    pub fn new_used_symbol(used_symbol: Ident) -> ModuleSymbols {
        Self {
            used_symbols: HashSet::from([used_symbol]),
            ..Default::default()
        }
    }

    pub fn merge_iter<Iter: IntoIterator<Item = ModuleSymbols>>(
        self,
        analyzed_modules: Iter,
    ) -> ModuleSymbols {
        merge_iter(analyzed_modules).merge(self)
    }

    pub fn merge(self, analyzed_module: ModuleSymbols) -> Self {
        let mut defined_symbols = self.defined_symbols;
        defined_symbols.extend(analyzed_module.defined_symbols);

        let mut used_symbols = self.used_symbols;
        used_symbols.extend(analyzed_module.used_symbols);

        let mut exported_symbols = self.exported_symbols;
        exported_symbols.extend(analyzed_module.exported_symbols);

        let mut imported_symbols = self.imported_symbols;
        imported_symbols.extend(analyzed_module.imported_symbols);

        Self {
            defined_symbols,
            used_symbols,
            exported_symbols,
            imported_symbols,
        }
    }

    pub fn defined_to_exported(self) -> Self {
        let mut exported_symbols = self.exported_symbols;
        exported_symbols.extend(self.defined_symbols);

        Self {
            defined_symbols: HashSet::new(),
            exported_symbols,
            used_symbols: self.used_symbols,
            imported_symbols: self.imported_symbols,
        }
    }
}

pub fn merge_iter<Iter: IntoIterator<Item = ModuleSymbols>>(
    analyzed_modules: Iter,
) -> ModuleSymbols {
    let mut analyzed_module = ModuleSymbols::default();

    for other_analyzed_module in analyzed_modules {
        analyzed_module = analyzed_module.merge(other_analyzed_module);
    }

    analyzed_module
}
