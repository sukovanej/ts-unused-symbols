mod analyze_package;
mod analyzed_module;
mod analyzer;
mod module_symbols;

use std::path::PathBuf;

use crate::{analyze_package::analyze_package, analyzer::analyze_file};

fn main() {
    //let analyze_package = analyze_package(PathBuf::from("example/src"));
    let analyzed_module = analyze_file(PathBuf::from("example/src/example.js"));
    println!("{}", analyzed_module.debug());
}
