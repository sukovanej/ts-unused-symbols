mod analyze_package;
mod analyzed_module;
mod analyzer;
mod find_unused_symbols;
mod module_symbols;

use std::path::PathBuf;

use crate::{analyze_package::analyze_package, find_unused_symbols::find_unused_symbols};

fn main() {
    let analyzed_package = analyze_package(PathBuf::from("example/src"));
    let unused_symbols = find_unused_symbols(&analyzed_package);
    println!("{:#?}", unused_symbols);
}
