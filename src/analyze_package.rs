use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use crate::analyze_file::analyze_file;
use crate::analyzed_module::AnalyzedModule;
use crate::module_symbols::{Export, ImportedSymbol, ModuleSymbols, Reexport};

#[derive(Clone, Debug)]
pub struct AnalyzedPackage<P> {
    pub path: P,
    pub modules: Vec<AnalyzedModule<P>>,
}

// pub struct AnalyzeOptions {}

pub fn analyze_package(path: PathBuf) -> AnalyzedPackage<PathBuf> {
    let paths = traverse_path(&path);
    let modules = paths
        .into_iter()
        .map(analyze_module_with_path_resolve)
        .collect();

    AnalyzedPackage { path, modules }
}

fn analyze_module_with_path_resolve(path: PathBuf) -> AnalyzedModule<PathBuf> {
    let analyzed_file = analyze_file(path.clone());

    AnalyzedModule {
        path: path.canonicalize().unwrap(),
        symbols: ModuleSymbols {
            exports: analyzed_file
                .symbols
                .exports
                .iter()
                .map(|export| match export {
                    Export::Default => Export::Default,
                    Export::Symbol(s) => Export::Symbol(s.to_owned()),
                    Export::AllFrom(s) => Export::AllFrom(resolve_import_path(&path, s)),
                    Export::Reexport(e) => Export::Reexport(Reexport {
                        from: resolve_import_path(&path, &e.from),
                    }),
                })
                .collect(),
            imports: analyzed_file
                .symbols
                .imports
                .iter()
                .map(|import| ImportedSymbol {
                    symbols: import.symbols.clone(),
                    from: resolve_import_path(&path, &import.from),
                })
                .collect(),
        },
    }
}

fn traverse_path(path: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    let dir = fs::read_dir(path).unwrap();

    for file in dir {
        let file = file.unwrap();

        let file_type = file.file_type().unwrap();

        if file_type.is_dir() {
            result.extend(traverse_path(&file.path()));
        } else if file_type.is_file() {
            result.push(file.path());
        }
    }

    result
}

fn resolve_import_path(current_path: &Path, import_str: &str) -> PathBuf {
    let mut path = current_path.to_owned();
    path.pop();
    path.push(PathBuf::from(import_str));
    path.set_extension("ts");
    path.canonicalize().unwrap()
}
