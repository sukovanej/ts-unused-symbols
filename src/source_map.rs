use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SourceMap {
    pub sources: Vec<String>,
}

pub fn try_load_source_map(path: &Path) -> Option<SourceMap> {
    if !path.exists() {
        return None;
    }

    let source_map_str = fs::read_to_string(path).unwrap();
    let source_map = serde_json::from_str(&source_map_str);

    Some(source_map.unwrap())
}
