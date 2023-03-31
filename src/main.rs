mod analyze_file;
mod analyze_package;
mod analyze_plan;
mod analyze_symbols_usage;
mod analyzed_module;
mod find_unused_exports;
mod module_symbols;
mod package_json;
mod resolve_import_path;
mod source_map;
mod tsconfig;

use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use find_unused_exports::UnusedExport;
use regex::Regex;

use crate::analyze_package::{analyze_package, AnalyzeOptions, AnalyzedPackage};
use crate::analyze_plan::prepare_analyze_plan;
use crate::find_unused_exports::{find_unused_exports, Symbol};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, help = "Include into analysis but ignore unused symbols")]
    ignore_patterns: Vec<String>,

    #[arg(short, long, help = "Completely exclude from the analysis")]
    exclude_patterns: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let path = PathBuf::from(args.path);

    let mut exclude_patterns = vec!["node_modules".to_string()];
    exclude_patterns.extend(args.exclude_patterns);

    let mut ignore_patterns = vec!["node_modules".to_string()];
    ignore_patterns.extend(args.ignore_patterns);

    let options = AnalyzeOptions::new(
        parse_regex_item(ignore_patterns.into_iter()),
        parse_regex_item(exclude_patterns.into_iter()),
    );

    let analyze_plan = prepare_analyze_plan(&path);

    let analyzed_packages = analyze_plan
        .packages
        .iter()
        .map(|package| {
            analyze_package(
                &package.path,
                &package.tsconfig,
                &options,
                &analyze_plan.packages,
            )
        })
        .collect::<Vec<AnalyzedPackage>>();

    let unused_exports = find_unused_exports(&analyzed_packages);
    let final_unused_exports = filter_ignored(&unused_exports, &options.ignore_patterns);
    let number_of_ignored = unused_exports.len() - final_unused_exports.len();
    let number_of_files = analyzed_packages
        .iter()
        .map(|p| p.modules.len())
        .sum::<usize>();

    print_unsed_exports(&final_unused_exports);

    println!();
    println!(" - {} unused exports", final_unused_exports.len());
    println!(" - {number_of_ignored} unused exports ignored in the report",);
    println!(" - {number_of_files} files analyzed");
}

fn parse_regex_item<I: Iterator<Item = String>>(i: I) -> Vec<Regex> {
    i.map(|p| Regex::new(&p).unwrap()).collect()
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
                    .map(|e| format!(
                        " - \x1b[93m{}\x1b[0m",
                        match &e.symbol {
                            Symbol::Symbol(s) => s.to_owned(),
                            Symbol::Default => "DEFAULT".into(),
                        }
                    ))
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    println!("{unused_exports_stdout}");
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
