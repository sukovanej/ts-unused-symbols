use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::package_json::{try_load_package_json, PackageJson};
use crate::tsconfig::{try_load_tsconfig, TsConfig};

#[derive(Debug, Clone)]
pub struct AnalyzePlan {
    pub packages: Vec<Package>,
}

impl AnalyzePlan {
    pub fn new(packages: Vec<Package>) -> Self {
        Self { packages }
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub path: PathBuf,
    pub package_json: PackageJson,
    pub tsconfig: Option<TsConfig>,
}

impl Package {
    pub fn new(path: &Path, package_json: PackageJson, tsconfig: Option<TsConfig>) -> Self {
        Self {
            path: path.to_owned(),
            package_json,
            tsconfig,
        }
    }
}

pub fn prepare_analyze_plan(path: &Path) -> Result<AnalyzePlan> {
    let package_json = try_load_package_json(path)
        .with_context(|| format!("package.json in {:?} not found", path))?;
    let mut packages = vec![];

    if let Some(monorepo_packages) = package_json.workspaces {
        for monorepo_package_wildcard in monorepo_packages {
            packages.extend(find_packages(path, &monorepo_package_wildcard));
        }
    } else {
        let tsconfig = try_load_tsconfig(path);
        packages = vec![Package::new(path, package_json, tsconfig)];
    }

    Ok(AnalyzePlan::new(packages))
}

fn find_packages(path: &Path, wildcard: &str) -> Vec<Package> {
    let package_paths = get_paths_matching_wildcard(path, wildcard);
    let mut packages = vec![];

    for package_path in package_paths {
        packages.push(get_package(&package_path));
    }

    packages
}

fn get_paths_matching_wildcard(path: &Path, wildcard: &str) -> Vec<PathBuf> {
    let path_parts = wildcard
        .split('/')
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();

    // currently supports only "<folder-name>/*" pattern

    if path_parts.len() == 2 && path_parts[1] == "*" && !path_parts[0].contains('*') {
        let mut path = path.to_owned();
        path.push(PathBuf::from(path_parts[0].clone()));

        return fs::read_dir(path)
            .unwrap()
            .map(|f| f.unwrap())
            .filter(|f| f.file_type().unwrap().is_dir())
            .map(|f| f.path())
            .collect();
    }

    todo!("{path:?}")
}

fn get_package(path: &Path) -> Package {
    let package_json = try_load_package_json(path).unwrap();
    let tsconfig = try_load_tsconfig(path);

    Package::new(path, package_json, tsconfig)
}
