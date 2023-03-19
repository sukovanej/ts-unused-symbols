mod analyze_file;
mod analyze_package;
mod analyzed_module;
mod find_unused_exports;
mod module_symbols;
mod tsconfig;

use std::path::PathBuf;

use clap::Parser;

use crate::{analyze_package::analyze_package, find_unused_exports::find_unused_exports, tsconfig::try_load_tsconfig};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,
}

fn main() {
    let args = Args::parse();
    let path = PathBuf::from(args.path);

    let tsconfig = try_load_tsconfig(&path);

    println!("{:?}", tsconfig);

    let analyzed_package = analyze_package(&path, &tsconfig);
    let unused_symbols = find_unused_exports(&analyzed_package)
        .iter()
        .map(|export| {
            format!(
                " - {}:{}",
                export.filename.to_str().unwrap().to_owned(),
                export.symbol
            )
        })
        .collect::<Vec<String>>()
        .join("\n");
    println!("{unused_symbols}");
}
