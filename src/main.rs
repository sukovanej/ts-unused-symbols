mod analyze_package;
mod analyzed_module;
mod analyzer;
mod module_symbols;

use std::path::PathBuf;

use crate::analyze_package::analyze_package;

fn main() {
    let analyzed_module = analyze_package(PathBuf::from("example"));
    println!("{}", analyzed_module.debug());
}
