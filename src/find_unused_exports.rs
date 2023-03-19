use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::module_symbols::ModuleSymbols;
use crate::{
    analyze_package::AnalyzedPackage,
    module_symbols::{Export, Import},
};

#[derive(Debug)]
pub struct UnusedExport {
    pub filename: PathBuf,
    pub symbol: String,
}

impl UnusedExport {
    fn new(path: &Path, symbol: String) -> Self {
        Self {
            filename: path.to_owned(),
            symbol,
        }
    }
}

pub fn find_unused_exports(analyzed_package: &AnalyzedPackage) -> Vec<UnusedExport> {
    let all_imports = get_all_imports(analyzed_package);
    let all_exports = get_all_exports(analyzed_package);
    let not_imported_exports = all_exports.difference(&all_imports);
    not_imported_exports
        .into_iter()
        .map(|(symbol, path)| UnusedExport::new(path, symbol.to_owned()))
        .collect()
}

fn get_all_imports(analyzed_package: &AnalyzedPackage) -> HashSet<(String, PathBuf)> {
    analyzed_package
        .modules
        .iter()
        .flat_map(|(_, module)| {
            module
                .symbols
                .imports
                .iter()
                .flat_map(|import| {
                    import
                        .symbols
                        .iter()
                        .flat_map(|symbol| {
                            resolve_import(symbol, &import.from, &module.symbols, analyzed_package)
                        })
                        .collect::<HashSet<(String, PathBuf)>>()
                })
                .collect::<HashSet<(String, PathBuf)>>()
        })
        .collect()
}

fn resolve_import(
    import: &Import,
    from: &Path,
    module_symbols: &ModuleSymbols<PathBuf>,
    analyzed_package: &AnalyzedPackage,
) -> HashSet<(String, PathBuf)> {
    let imports = match import {
        Import::Named(s) => {
            let imported_module = analyzed_package.modules.get(from);
            let mut resolved = HashSet::from([(s.to_owned(), from.to_owned())]);

            if let Some(imported_module) = imported_module {
                for export in &imported_module.symbols.exports {
                    match export {
                        Export::Symbol(exported_symbol) => {
                            if s == exported_symbol {
                                break;
                            }
                        }
                        Export::Reexport(r) => todo!("{r:?}"),
                        Export::AllFrom(from) => {
                            let module = analyzed_package.modules.get(from).unwrap();

                            if module.exports_symbol(s.as_ref()) {
                                resolved.insert((s.to_owned(), from.to_owned()));
                            }
                        }
                        Export::Default => todo!("default export"),
                    }
                }
            }

            resolved
        }
        Import::Default(i) => HashSet::new(), // TODO
        Import::Namespace(alias) => module_symbols
            .usages
            .iter()
            .filter_map(|usage| match usage {
                crate::module_symbols::Usage::Namespace(symbol, current_alias) => {
                    if current_alias == alias {
                        Some(symbol)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .map(|symbol| (symbol.to_owned(), from.to_owned()))
            .collect(),
    };

    imports
}

fn get_all_exports(analyzed_package: &AnalyzedPackage) -> HashSet<(String, PathBuf)> {
    analyzed_package
        .modules
        .iter()
        .flat_map(|(path, module)| {
            module
                .symbols
                .exports
                .iter()
                .filter_map(|export| match export {
                    Export::Default => None,
                    Export::Reexport(_) => None,
                    Export::AllFrom(_) => None,
                    Export::Symbol(s) => Some((s.to_owned(), path.to_owned())),
                })
                .collect::<HashSet<(String, PathBuf)>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{analyze_package::analyze_package, find_unused_exports::find_unused_exports};

    #[test]
    fn relative_imports() {
        let analyzed_package = analyze_package(&PathBuf::from("./tests/relative-imports/"), &None);
        let unused_exports = find_unused_exports(&analyzed_package);
        assert_eq!(unused_exports.len(), 0);
    }

    #[test]
    fn reexported_symbols() {
        let analyzed_package =
            analyze_package(&PathBuf::from("./tests/reexported-symbols/"), &None);
        let unused_exports = find_unused_exports(&analyzed_package);
        assert_eq!(unused_exports.len(), 0);
    }

    #[test]
    fn namespace_import() {
        let analyzed_package = analyze_package(&PathBuf::from("./tests/namespace-imports/"), &None);
        let unused_exports = find_unused_exports(&analyzed_package);
        assert_eq!(unused_exports.len(), 0);
    }
}
