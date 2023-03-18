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
                    Export::AllFrom(s) => {
                        let mut path = path.clone();
                        path.pop();
                        path.push(PathBuf::from(s));
                        path.set_extension("ts");
                        Export::AllFrom(path.canonicalize().unwrap())
                    }
                    Export::Reexport(e) => Export::Reexport(Reexport {
                        from: {
                            let mut path = path.clone();
                            path.pop();
                            path.push(PathBuf::from(e.from.clone()));
                            path.set_extension("ts");
                            path.canonicalize().unwrap()
                        },
                    }),
                })
                .collect(),
            imports: analyzed_file
                .symbols
                .imports
                .iter()
                .map(|import| ImportedSymbol {
                    symbols: import.symbols.clone(),
                    from: {
                        let mut path = path.clone();
                        path.pop();
                        path.push(PathBuf::from(import.from.clone()));
                        path.set_extension("ts");
                        path.canonicalize()
                            .expect(&format!("{} not found", import.from))
                    },
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
