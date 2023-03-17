use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{analyzed_module::AnalyzedModule, analyzer::analyze_file};

#[derive(Clone)]
pub struct AnalyzedPackage {
    pub path: PathBuf,
    pub modules: Vec<AnalyzedModule>,
}

impl AnalyzedPackage {
    pub fn debug(&self) -> String {
        let path = self.path.to_str().unwrap();
        let modules: Vec<String> = self
            .modules
            .clone()
            .into_iter()
            .map(|m| m.debug())
            .collect();

        format!("Package: {}\n{:?}", path, modules)
    }
}

pub struct AnalyzeOptions {}

pub fn analyze_package(path: PathBuf) -> AnalyzedPackage {
    let paths = traverse_path(&path);

    let modules = paths.into_iter().map(analyze_file).collect();

    AnalyzedPackage { path, modules }
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
