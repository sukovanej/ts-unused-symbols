use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;

use crate::analyze_file::analyze_file;
use crate::analyze_plan::Package;
use crate::analyzed_module::AnalyzedModule;
use crate::module_symbols::{Export, ImportedSymbol, ModuleSymbols};
use crate::resolve_import_path::resolve_import_path;
use crate::tsconfig::TsConfig;

#[derive(Clone, Debug)]
pub struct AnalyzedPackage {
    pub path: PathBuf,
    pub modules: HashMap<PathBuf, AnalyzedModule<PathBuf>>,
    pub unresolved_paths: HashSet<String>,
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
    packages: &[Package],
) -> Result<AnalyzedPackage> {
    let build_path = tsconfig
        .clone()
        .and_then(|c| c.compiler_options)
        .and_then(|c| c.out_dir)
        .map(|c| {
            let mut path = path.to_owned();
            path.push(c);
            path.canonicalize().unwrap()
        });

    let mut unresolved_paths = HashSet::new();

    let paths = traverse_path(path, &options.exclude_patterns, &build_path);
    let modules = paths
        .into_iter()
        .map(|p| {
            analyze_module_with_path_resolve(&p, tsconfig, path, packages, &mut unresolved_paths)
                .map(|m| (p.to_owned(), m))
        })
        .collect::<Result<_>>()?;

    Ok(AnalyzedPackage {
        path: path.to_owned(),
        modules,
        unresolved_paths,
    })
}

fn analyze_module_with_path_resolve(
    path: &Path,
    tsconfig: &Option<TsConfig>,
    package_path: &Path,
    packages: &[Package],
    unresolved_paths: &mut HashSet<String>,
) -> Result<AnalyzedModule<PathBuf>> {
    let analyzed_file = analyze_file(path);

    let exports = analyzed_file
        .symbols
        .exports
        .iter()
        .map(|export| match export {
            Export::Default => Ok(Some(Export::Default)),
            Export::Symbol(s) => Ok(Some(Export::Symbol(s.to_owned()))),
            Export::AllFrom(s) => {
                let resolved_import_path =
                    resolve_import_path(path, s, tsconfig, package_path, packages);

                if resolved_import_path.as_ref().unwrap_or(&None).is_none() {
                    unresolved_paths.insert(s.to_owned());
                }

                resolved_import_path.map(|i| i.map(Export::AllFrom))
            }
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    let imports = analyzed_file
        .symbols
        .imports
        .iter()
        .map(|import| {
            let resolved_import_path =
                resolve_import_path(path, &import.from, tsconfig, package_path, packages);

            if resolved_import_path.as_ref().unwrap_or(&None).is_none() {
                unresolved_paths.insert(import.from.to_owned());
            }

            resolved_import_path.map(|from| {
                from.map(|from| ImportedSymbol {
                    symbols: import.symbols.clone(),
                    from,
                })
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(AnalyzedModule {
        path: path.canonicalize().unwrap(),
        symbols: ModuleSymbols {
            usages: analyzed_file.symbols.usages,
            exports,
            imports,
        },
    })
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

    use anyhow::Result;

    use crate::{
        analyze_package::analyze_package,
        module_symbols::{Import, ImportedSymbol},
    };

    #[test]
    fn namespace_imports() -> Result<()> {
        let analyzed_module = analyze_package(
            &PathBuf::from("./tests/namespace-imports/"),
            &Default::default(),
            &Default::default(),
            Default::default(),
        )?;
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

        Ok(())
    }
}
