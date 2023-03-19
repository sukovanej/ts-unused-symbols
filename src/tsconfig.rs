use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TsConfig {
    pub compiler_options: Option<TsConfigCompilerOptions>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TsConfigCompilerOptions {
    pub base_url: Option<String>,
    pub out_dir: Option<PathBuf>,
}

pub fn try_load_tsconfig(path: &Path) -> Option<TsConfig> {
    let tsconfig_filename = "tsconfig.json";

    let mut tsconfig_path = path.to_owned();
    tsconfig_path.push(PathBuf::from(tsconfig_filename));

    if !tsconfig_path.exists() {
        return None;
    }

    let tsconfig_str = fs::read_to_string(tsconfig_path).unwrap();
    let tsconfig = serde_json::from_str(&tsconfig_str);

    Some(tsconfig.unwrap())
}
