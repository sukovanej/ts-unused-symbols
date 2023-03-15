use std::collections::HashSet;

use swc_ecma_ast::Ident;

#[derive(Debug, Clone)]
pub struct ModuleSymbols {
    pub defined_symbols: HashSet<Ident>,
    pub used_symbols: HashSet<Ident>,
    pub exported_symbols: HashSet<Ident>,
}

impl Default for ModuleSymbols {
    fn default() -> Self {
        return Self {
            defined_symbols: HashSet::new(),
            exported_symbols: HashSet::new(),
            used_symbols: HashSet::new(),
        };
    }
}

impl ModuleSymbols {
    pub fn debug(&self) -> String {
        let defined_symbols: Vec<String> = self
            .defined_symbols
            .clone()
            .into_iter()
            .map(|i| i.to_string())
            .collect();

        let used_symbols: Vec<String> = self
            .used_symbols
            .clone()
            .into_iter()
            .map(|i| i.to_string())
            .collect();

        let exported_symbols: Vec<String> = self
            .exported_symbols
            .clone()
            .into_iter()
            .map(|i| i.to_string())
            .collect();

        return format!(
            "Defined symbols:\n{:?}\nUsed symbols:\n{:?}\nExported symbols\n{:?}",
            defined_symbols, used_symbols, exported_symbols
        );
    }

    pub fn new_defined_symbol(defined_symbol: Ident) -> ModuleSymbols {
        return Self {
            defined_symbols: HashSet::from([defined_symbol]),
            exported_symbols: HashSet::new(),
            used_symbols: HashSet::new(),
        };
    }

    pub fn new_used_symbol(used_symbol: Ident) -> ModuleSymbols {
        return Self {
            defined_symbols: HashSet::new(),
            exported_symbols: HashSet::new(),
            used_symbols: HashSet::from([used_symbol]),
        };
    }

    pub fn merge_iter<Iter: IntoIterator<Item = ModuleSymbols>>(
        self,
        analyzed_modules: Iter,
    ) -> ModuleSymbols {
        merge_iter(analyzed_modules).merge(self)
    }

    pub fn merge(self, analyzed_module: ModuleSymbols) -> ModuleSymbols {
        let mut defined_symbols = self.defined_symbols;
        defined_symbols.extend(analyzed_module.defined_symbols);

        let mut used_symbols = self.used_symbols;
        used_symbols.extend(analyzed_module.used_symbols);

        let mut exported_symbols = self.exported_symbols;
        exported_symbols.extend(analyzed_module.exported_symbols);

        ModuleSymbols {
            defined_symbols,
            used_symbols,
            exported_symbols,
        }
    }

    pub fn defined_to_exported(self) -> Self {
        let mut exported_symbols = self.exported_symbols;
        exported_symbols.extend(self.defined_symbols);

        Self {
            defined_symbols: HashSet::new(),
            exported_symbols,
            used_symbols: self.used_symbols,
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

    return analyzed_module;
}
