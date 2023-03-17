mod analyze_package;
mod analyzed_module;
mod analyzer;
mod module_symbols;

use std::path::PathBuf;

use crate::analyze_package::analyze_package;

fn main() {
    let analyzed_package = analyze_package(PathBuf::from("example/src"));
    println!("{}", analyzed_package.debug());
}
