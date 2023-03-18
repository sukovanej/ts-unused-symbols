use std::path::{Path, PathBuf};

use crate::{
    analyze_package::AnalyzedPackage,
    analyzed_module::AnalyzedModule,
    module_symbols::{Export, Import},
};

#[derive(Debug)]
pub struct UnusedExport {
    pub filename: PathBuf,
    pub symbol: String,
}

pub fn find_unused_exports(analyzed_package: &AnalyzedPackage<PathBuf>) -> Vec<UnusedExport> {
    let mut unused_exports: Vec<UnusedExport> = vec![];
    let modules = &analyzed_package.modules;

    for i in 0..modules.len() {
        let module = &modules[i];
        let unused_in_module = find_unused_exports_in_module(
            module,
            &modules
                .iter()
                .enumerate()
                .filter_map(|(j, v)| Some(v).filter(|_| j != i))
                .collect(),
        );

        unused_exports.extend(
            unused_in_module
                .iter()
                .filter_map(|export| match export {
                    Export::Symbol(symbol) => Some(UnusedExport {
                        filename: module.path.to_owned(),
                        symbol: symbol.to_string(),
                    }),
                    Export::AllFrom(_) => None, // TODO
                    i => todo!("{:?}", i),
                })
                .collect::<Vec<UnusedExport>>(),
        );
    }

    unused_exports
}

fn find_unused_exports_in_module(
    analyzed_module: &AnalyzedModule<PathBuf>,
    other_modules: &Vec<&AnalyzedModule<PathBuf>>,
) -> Vec<Export<PathBuf>> {
    let mut unused_exports: Vec<Export<PathBuf>> = vec![];

    for export in &analyzed_module.symbols.exports {
        if !is_export_used_in_module(export, &analyzed_module.path, other_modules) {
            unused_exports.push(export.to_owned());
        }
    }

    unused_exports
}

fn is_export_used_in_module(
    export: &Export<PathBuf>,
    module_path: &Path,
    other_modules: &Vec<&AnalyzedModule<PathBuf>>,
) -> bool {
    for other_module in other_modules {
        for import in &other_module.symbols.imports {
            if import.from != module_path {
                continue;
            }

            for imported_symbol in &import.symbols {
                match (imported_symbol, export) {
                    (Import::Named(imported_symbol), Export::Symbol(symbol)) => {
                        if imported_symbol == symbol {
                            return true;
                        }
                    }
                    (Import::Namespace, _) => {
                        return true; // TODO need to check which symbols are used precisely
                    }
                    (Import::Named(_), _) => continue,
                    (i, e) => todo!("{:#?}\n---\n{:#?}", i, e),
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{analyze_package::analyze_package, find_unused_exports::find_unused_exports};

    #[test]
    fn relative_imports() {
        let analyzed_package = analyze_package(PathBuf::from("./tests/relative-imports/"));
        let unused_exports = find_unused_exports(&analyzed_package);
        println!("{:#?}", analyzed_package);
        assert_eq!(unused_exports.len(), 0);
    }
}
