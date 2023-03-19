mod analyze_file;
mod analyze_package;
mod analyze_symbols_usage;
mod analyzed_module;
mod find_unused_exports;
mod module_symbols;
mod tsconfig;

use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use find_unused_exports::UnusedExport;
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

    #[arg(short, long)]
    exclude_patterns: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let path = PathBuf::from(args.path);

    let tsconfig = try_load_tsconfig(&path);

    let mut exclude_patterns = vec![Regex::new("node_modules").unwrap()];
    exclude_patterns.extend(args.exclude_patterns.iter().map(|p| Regex::new(p).unwrap()));

    let mut ignore_patterns = vec![Regex::new("node_modules").unwrap()];
    ignore_patterns.extend(args.ignore_patterns.iter().map(|p| Regex::new(p).unwrap()));

    let options = AnalyzeOptions::new(ignore_patterns, exclude_patterns, tsconfig);

    let analyzed_package = analyze_package(&path, &options);
    let unused_exports = find_unused_exports(&analyzed_package);
    let final_unused_exports = filter_ignored(&unused_exports, &options.ignore_patterns);
    print_unsed_exports(&final_unused_exports);

    let number_of_ignored = unused_exports.len() - final_unused_exports.len();
    println!("{number_of_ignored} unused exports ignored in the report");
}

fn filter_ignored(unused_exports: &[UnusedExport], ignore_patterns: &[Regex]) -> Vec<UnusedExport> {
    unused_exports
        .iter()
        .filter(|e| {
            !ignore_patterns
                .iter()
                .any(|r| r.is_match(e.filename.to_str().unwrap()))
        })
        .cloned()
        .collect()
}

fn print_unsed_exports(unused_exports: &[UnusedExport]) {
    let unused_exports_stdout = group_by_path(unused_exports)
        .iter()
        .map(|(path, exports)| {
            format!(
                "{}:\n{}",
                path.to_str().unwrap(),
                exports
                    .iter()
                    .map(|e| format!(" - \x1b[93m{}\x1b[0m", e.symbol))
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    println!("{unused_exports_stdout}");
    println!("Found {} unused exports", unused_exports.len());
}

fn group_by_path(unused_exports: &[UnusedExport]) -> HashMap<PathBuf, Vec<UnusedExport>> {
    let mut result = HashMap::new();

    for unused_export in unused_exports {
        if !result.contains_key(&unused_export.filename) {
            result.insert(
                unused_export.filename.to_owned(),
                vec![unused_export.to_owned()],
            );
        } else {
            result
                .get_mut(&unused_export.filename)
                .unwrap()
                .push(unused_export.to_owned());
        }
    }

    result
}
