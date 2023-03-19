mod analyze_file;
mod analyze_package;
mod analyze_symbols_usage;
mod analyzed_module;
mod find_unused_exports;
mod module_symbols;
mod tsconfig;

use std::path::PathBuf;

use clap::Parser;
use regex::Regex;

use crate::{
    analyze_package::{analyze_package, AnalyzeOptions},
    find_unused_exports::find_unused_exports,
    tsconfig::try_load_tsconfig,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long)]
    ignore_patterns: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let path = PathBuf::from(args.path);

    let tsconfig = try_load_tsconfig(&path);
    let mut ignore_patterns = vec![Regex::new("node_modules").unwrap()];
    ignore_patterns.extend(args.ignore_patterns.iter().map(|p| Regex::new(p).unwrap()));

    let options = AnalyzeOptions::new(ignore_patterns, tsconfig);

    println!("options: {:?}", options);

    let analyzed_package = analyze_package(&path, &options);
    let unused_symbols = find_unused_exports(&analyzed_package);
    let unused_symbols_stdout = unused_symbols
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
    println!("{unused_symbols_stdout}");
    println!("Found {} unused exports", unused_symbols.len());
}
