use std::path::{Path, PathBuf};

use crate::{analyze_package::AnalyzedPackage, analyzed_module::AnalyzedModule};

#[derive(Debug)]
pub enum UnusedSymbol {
    UnusedExportedSymbol(UnusedExportedSymbol),
}

#[derive(Debug)]
pub struct UnusedExportedSymbol {
    pub filename: PathBuf,
    pub symbol: String,
}

pub fn find_unused_symbols(analyzed_package: &AnalyzedPackage) -> Vec<UnusedSymbol> {
    let mut unused_symbols: Vec<UnusedSymbol> = vec![];
    let modules = &analyzed_package.modules;

    for i in 0..modules.len() {
        let module = &modules[i];
        let unused_in_module = find_unused_symbols_in_module(
            module,
            &modules
                .iter()
                .enumerate()
                .filter_map(|(j, v)| Some(v).filter(|_| j != i))
                .collect(),
        );

        unused_symbols.extend(
            unused_in_module
                .iter()
                .map(|symbol| {
                    UnusedSymbol::UnusedExportedSymbol(UnusedExportedSymbol {
                        filename: module.path.to_owned(),
                        symbol: symbol.to_string(),
                    })
                })
                .collect::<Vec<UnusedSymbol>>(),
        );
    }

    unused_symbols
}

fn find_unused_symbols_in_module(
    analyzed_module: &AnalyzedModule,
    other_modules: &Vec<&AnalyzedModule>,
) -> Vec<String> {
    let mut unused_symbols: Vec<String> = vec![];

    for symbol in &analyzed_module.symbols.exported_symbols {
        if !is_symbol_used_in_module(symbol, &analyzed_module.path, other_modules) {
            unused_symbols.push(symbol.to_owned());
        }
    }

    unused_symbols
}

fn is_symbol_used_in_module(
    symbol: &str,
    module_path: &Path,
    other_modules: &Vec<&AnalyzedModule>,
) -> bool {
    for other_module in other_modules {
        for imported_symbol in &other_module.symbols.imported_symbols {
            let resolved_path = resolve_path(&imported_symbol.from, module_path);

            if resolved_path == module_path && imported_symbol.symbols.contains(&symbol.to_owned())
            {
                return true;
            }
        }
    }

    false
}

fn resolve_path(from: &str, module_path: &Path) -> PathBuf {
    let mut module_dir = module_path.to_owned();
    module_dir.pop();
    let mut module_dir = module_dir.join(PathBuf::from(from));
    module_dir.set_extension("ts"); // TODO: set extension from `from`
    module_dir
}
