use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    pub name: String,
    pub workspaces: Option<Vec<String>>,
}

pub fn try_load_package_json(path: &Path) -> Option<PackageJson> {
    let package_json_filename = "package.json";

    let mut package_json_path = path.to_owned();
    package_json_path.push(PathBuf::from(package_json_filename));

    if !package_json_path.exists() {
        return None;
    }

    let package_json_str = fs::read_to_string(package_json_path).unwrap();
    let package_json = serde_json::from_str(&package_json_str);

    Some(package_json.unwrap())
}
