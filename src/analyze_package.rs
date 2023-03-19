use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use crate::analyze_file::analyze_file;
use crate::analyzed_module::AnalyzedModule;
use crate::module_symbols::{Export, ImportedSymbol, ModuleSymbols, Reexport};
use crate::tsconfig::TsConfig;

#[derive(Clone, Debug)]
pub struct AnalyzedPackage {
    pub path: PathBuf,
    pub modules: HashMap<PathBuf, AnalyzedModule<PathBuf>>,
}

// pub struct AnalyzeOptions {}

pub fn analyze_package(path: &Path, tsconfig: &Option<TsConfig>) -> AnalyzedPackage {
    let mut source_files_path = path.to_owned();
    source_files_path.push("src");

    let paths = traverse_path(&source_files_path);
    let modules = paths
        .into_iter()
        .map(|p| (p.to_owned(), analyze_module_with_path_resolve(&p, tsconfig, path)))
        .collect();

    AnalyzedPackage {
        path: path.to_owned(),
        modules,
    }
}

fn analyze_module_with_path_resolve(
    path: &Path,
    tsconfig: &Option<TsConfig>,
    package_path: &Path,
) -> AnalyzedModule<PathBuf> {
    let analyzed_file = analyze_file(path);

    AnalyzedModule {
        path: path.canonicalize().unwrap(),
        symbols: ModuleSymbols {
            exports: analyzed_file
                .symbols
                .exports
                .iter()
                .filter_map(|export| match export {
                    Export::Default => Some(Export::Default),
                    Export::Symbol(s) => Some(Export::Symbol(s.to_owned())),
                    Export::AllFrom(s) => {
                        resolve_import_path(&path, s, tsconfig, package_path).map(Export::AllFrom)
                    }
                    Export::Reexport(e) => {
                        resolve_import_path(&path, &e.from, tsconfig, package_path)
                            .map(|from| Export::Reexport(Reexport { from }))
                    }
                })
                .collect(),
            imports: analyzed_file
                .symbols
                .imports
                .iter()
                .filter_map(|import| {
                    resolve_import_path(&path, &import.from, tsconfig, package_path).map(|from| {
                        ImportedSymbol {
                            symbols: import.symbols.clone(),
                            from,
                        }
                    })
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
            let extension = file
                .path()
                .extension()
                .map(|s| s.to_str().unwrap().to_owned())
                .unwrap_or("".to_string());

            if vec!["ts"].contains(&extension.as_str()) {
                result.push(file.path().canonicalize().unwrap());
            }
        }
    }

    result
}

fn resolve_import_path(
    current_path: &Path,
    import_str: &str,
    tsconfig: &Option<TsConfig>,
    package_base_path: &Path,
) -> Option<PathBuf> {
    let mut path = current_path.to_owned();
    path.pop();

    let import_path = PathBuf::from(import_str);

    if import_str.starts_with('.') {
        path.push(import_path);
    } else if let Some(base_url) = tsconfig
        .to_owned()
        .and_then(|t| t.compiler_options)
        .and_then(|c| c.base_url)
    {
        path = package_base_path.to_owned();
        path.push(PathBuf::from(base_url));
        path.push(import_path);
    }

    if path.is_dir() {
        path.push(PathBuf::from("index"));
    }

    path.set_extension("ts");

    if !path.exists() {
        //println!("{import_str:?} not found");
        return None;
    }

    Some(path.canonicalize().expect(&format!(
        "Failed to resolve {} in {}",
        import_str.to_owned(),
        current_path.to_str().unwrap()
    )))
}
