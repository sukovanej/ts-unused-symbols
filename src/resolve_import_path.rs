use anyhow::{Context, Result};

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::analyze_plan::Package;
use crate::source_map::try_load_source_map;
use crate::tsconfig::TsConfig;

pub fn resolve_import_path(
    current_path: &Path,
    import_str: &str,
    tsconfig: &Option<TsConfig>,
    package_base_path: &Path,
    packages: &[Package],
) -> Result<Option<PathBuf>> {
    if let Some(path) = try_resolve_as_monorepo_package(import_str, packages)? {
        return Ok(Some(path));
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
    let filename = path
        .file_name()
        .context("Could not resolve file name")?
        .to_str()
        .context("Could not resolve file name")?
        .to_owned();

    while !path.exists() {
        if let Some(extension) = possible_extensions.pop_front() {
            let extension = extension;
            path.set_file_name(format!("{filename}.{extension}"));
        } else {
            break;
        }
    }

    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(path.canonicalize().with_context(|| {
        format!("Failed to resolve {} in {filename}", import_str.to_owned())
    })?))
}

/// This one needs a shit ton of refactoring
///
/// The idea is to try matching against a package name in the monorepo,
/// checking the package.json types field to find the imported file,
/// find corresponding .map file and parse source file from it.
fn try_resolve_as_monorepo_package(
    import_str: &str,
    packages: &[Package],
) -> Result<Option<PathBuf>> {
    for package in packages.iter() {
        let package_name = &package.package_json.name;
        if !import_str.starts_with(package_name) {
            continue;
        }

        let mut final_path = package.path.to_owned();

        if import_str == package_name {
            let types = package.package_json.types.to_owned().context("")?;
            final_path.push(format!("{types}.map"));
        } else {
            let mut rest_path = &import_str[package.package_json.name.len()..];

            if rest_path.starts_with('/') {
                rest_path = &rest_path[1..];
            }
            final_path.push(format!("{rest_path}.d.ts.map"));
        }

        if let Some(source_map) = try_load_source_map(&final_path) {
            if source_map.sources.len() != 1 {
                panic!("Unexpected source {:?}", source_map.sources);
            }

            final_path.pop();
            final_path.push(&source_map.sources[0]);
        }

        final_path = final_path.canonicalize().unwrap();

        if !final_path.exists() {
            panic!("Path {final_path:?} doesnt exist");
        }

        return Ok(Some(final_path));
    }

    Ok(None)
}
