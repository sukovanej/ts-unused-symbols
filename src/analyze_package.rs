use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::analyze_file::analyze_file;
use crate::analyze_plan::MonorepoImportMapping;
use crate::analyzed_module::AnalyzedModule;
use crate::module_symbols::{Export, ImportedSymbol, ModuleSymbols, Reexport};
use crate::resolve_import_path::resolve_import_path;
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
}

impl AnalyzeOptions {
    pub fn new(ignore_patterns: Vec<Regex>, exclude_patterns: Vec<Regex>) -> Self {
        Self {
            ignore_patterns,
            exclude_patterns,
        }
    }
}

pub fn analyze_package(
    path: &Path,
    tsconfig: &Option<TsConfig>,
    options: &AnalyzeOptions,
    monorepo_import_mapping: &MonorepoImportMapping,
) -> AnalyzedPackage {
    let build_path = tsconfig
        .clone()
        .and_then(|c| c.compiler_options)
        .and_then(|c| c.out_dir)
        .map(|c| {
            let mut path = path.to_owned();
            path.push(c);
            path.canonicalize().unwrap()
        });

    let paths = traverse_path(path, &options.exclude_patterns, &build_path);
    let modules = paths
        .into_iter()
        .map(|p| {
            (
                p.to_owned(),
                analyze_module_with_path_resolve(&p, tsconfig, path, monorepo_import_mapping),
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
    monorepo_import_mapping: &MonorepoImportMapping,
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
                    Export::AllFrom(s) => resolve_import_path(
                        path,
                        s,
                        tsconfig,
                        package_path,
                        monorepo_import_mapping,
                    )
                    .map(Export::AllFrom),
                    Export::Reexport(e) => resolve_import_path(
                        path,
                        &e.from,
                        tsconfig,
                        package_path,
                        monorepo_import_mapping,
                    )
                    .map(|from| Export::Reexport(Reexport { from })),
                })
                .collect(),
            imports: analyzed_file
                .symbols
                .imports
                .iter()
                .filter_map(|import| {
                    resolve_import_path(
                        path,
                        &import.from,
                        tsconfig,
                        package_path,
                        monorepo_import_mapping,
                    )
                    .map(|from| ImportedSymbol {
                        symbols: import.symbols.clone(),
                        from,
                    })
                })
                .collect(),
        },
    }
}

fn traverse_path(
    path: &Path,
    exclude_patterns: &[Regex],
    out_dir: &Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut result = vec![];
    let dir = fs::read_dir(path).unwrap();

    for file in dir {
        let file = file.unwrap();

        let file_type = file.file_type().unwrap();
        let path = file.path().canonicalize().unwrap();

        if out_dir.as_ref().map(|i| i == &path).unwrap_or(false) {
            continue;
        }

        let path_str = path.to_str().unwrap();

        if file_type.is_dir() {
            if exclude_patterns.iter().any(|r| r.is_match(path_str)) {
                continue;
            }

            result.extend(traverse_path(&file.path(), exclude_patterns, out_dir));
        } else if file_type.is_file() {
            let extension = file
                .path()
                .extension()
                .map(|s| s.to_str().unwrap().to_owned())
                .unwrap_or("".to_string());

            if exclude_patterns.iter().any(|r| r.is_match(path_str)) {
                continue;
            }

            let possible_extensions = vec!["ts", "tsx", "js", "jsx", "mjs", "mts"];
            if possible_extensions.contains(&extension.as_str()) {
                result.push(path);
            }
        }
    }

    result
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
            &Default::default(),
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
