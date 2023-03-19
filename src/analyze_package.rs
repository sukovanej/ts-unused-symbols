use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::analyze_file::analyze_file;
use crate::analyzed_module::AnalyzedModule;
use crate::module_symbols::{Export, ImportedSymbol, ModuleSymbols, Reexport};
use crate::tsconfig::TsConfig;

#[derive(Clone, Debug)]
pub struct AnalyzedPackage {
    pub path: PathBuf,
    pub modules: HashMap<PathBuf, AnalyzedModule<PathBuf>>,
}

#[derive(Debug, Default)]
pub struct AnalyzeOptions {
    pub ignore_patterns: Vec<Regex>,
    pub exclude_patterns: Vec<Regex>,
    pub tsconfig: Option<TsConfig>,
}

impl AnalyzeOptions {
    pub fn new(
        ignore_patterns: Vec<Regex>,
        exclude_patterns: Vec<Regex>,
        tsconfig: Option<TsConfig>,
    ) -> Self {
        Self {
            ignore_patterns,
            exclude_patterns,
            tsconfig,
        }
    }
}

pub fn analyze_package(path: &Path, options: &AnalyzeOptions) -> AnalyzedPackage {
    let paths = traverse_path(path, &options.exclude_patterns);
    let modules = paths
        .into_iter()
        .map(|p| {
            (
                p.to_owned(),
                analyze_module_with_path_resolve(&p, &options.tsconfig, path),
            )
        })
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
            usages: analyzed_file.symbols.usages,
            exports: analyzed_file
                .symbols
                .exports
                .iter()
                .filter_map(|export| match export {
                    Export::Default => Some(Export::Default),
                    Export::Symbol(s) => Some(Export::Symbol(s.to_owned())),
                    Export::AllFrom(s) => {
                        resolve_import_path(path, s, tsconfig, package_path).map(Export::AllFrom)
                    }
                    Export::Reexport(e) => {
                        resolve_import_path(path, &e.from, tsconfig, package_path)
                            .map(|from| Export::Reexport(Reexport { from }))
                    }
                })
                .collect(),
            imports: analyzed_file
                .symbols
                .imports
                .iter()
                .filter_map(|import| {
                    resolve_import_path(path, &import.from, tsconfig, package_path).map(|from| {
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

fn traverse_path(path: &Path, exclude_patterns: &[Regex]) -> Vec<PathBuf> {
    let mut result = vec![];
    let dir = fs::read_dir(path).unwrap();

    for file in dir {
        let file = file.unwrap();

        let file_type = file.file_type().unwrap();
        let path = file.path().canonicalize().unwrap();
        let path_str = path.to_str().unwrap();

        if file_type.is_dir() {
            if exclude_patterns.iter().any(|r| r.is_match(path_str)) {
                continue;
            }

            result.extend(traverse_path(&file.path(), exclude_patterns));
        } else if file_type.is_file() {
            let extension = file
                .path()
                .extension()
                .map(|s| s.to_str().unwrap().to_owned())
                .unwrap_or("".to_string());

            if exclude_patterns.iter().any(|r| r.is_match(path_str)) {
                continue;
            }

            if vec!["ts"].contains(&extension.as_str()) {
                result.push(path);
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

    let mut possible_extensions = VecDeque::from(["ts", "tsx", "js", "jsx", "mjs", "mts"]);
    let filename = path.file_name().unwrap().to_str().unwrap().to_owned();

    while !path.exists() && !possible_extensions.is_empty() {
        let extension = possible_extensions.pop_front().unwrap();
        path.set_file_name(format!("{filename}.{extension}"));
    }

    if !path.exists() {
        println!("{import_str:?}, {path:#?} not found");
        return None;
    }

    Some(path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "Failed to resolve {} in {}",
            import_str.to_owned(),
            current_path.to_str().unwrap()
        )
    }))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::PathBuf};

    use crate::{
        analyze_package::analyze_package,
        module_symbols::{Import, ImportedSymbol},
    };

    #[test]
    fn namespace_imports() {
        let analyzed_module = analyze_package(
            &PathBuf::from("./tests/namespace-imports/"),
            &Default::default(),
        );
        assert_eq!(analyzed_module.modules.len(), 2);

        let module =
            &analyzed_module.modules[&PathBuf::from("./tests/namespace-imports/src/app.ts")
                .canonicalize()
                .unwrap()];

        assert_eq!(
            module.symbols.imports,
            HashSet::from([ImportedSymbol {
                from: PathBuf::from("./tests/namespace-imports/src/another.ts")
                    .canonicalize()
                    .unwrap(),
                symbols: vec![Import::Namespace("A".to_string())]
            }])
        );
    }
}
