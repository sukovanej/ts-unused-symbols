use std::{path::{Path, PathBuf}, collections::VecDeque};

use crate::{tsconfig::TsConfig, analyze_plan::MonorepoImportMapping};

pub fn resolve_import_path(
    current_path: &Path,
    import_str: &str,
    tsconfig: &Option<TsConfig>,
    package_base_path: &Path,
    monorepo_import_mapping: &MonorepoImportMapping,
) -> Option<PathBuf> {
    for (monorepo_name, package_path) in monorepo_import_mapping.iter() {
        if monorepo_name == import_str {
            let mut path = package_path.to_owned();
            path.push(PathBuf::from(import_str[monorepo_name.len()..].to_owned()));
            return Some(path);
        }

        if import_str.starts_with(monorepo_name) {
            let mut path = package_path.to_owned();
            path.push(PathBuf::from(import_str[monorepo_name.len()..].to_owned()));
            return Some(path);
        }
    }

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
