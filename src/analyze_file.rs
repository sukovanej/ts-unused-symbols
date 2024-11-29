use std::collections::HashSet;
use std::path::Path;

use swc_common::sync::Lrc;
use swc_common::SourceMap;
use swc_ecma_ast::{
    Decl, ExportSpecifier, Module, ModuleDecl, ModuleExportName, ModuleItem, Pat, TsModuleName,
};
use swc_ecma_ast::{EsVersion, ImportSpecifier};
use swc_ecma_parser::{error::Error, parse_file_as_module, Syntax, TsSyntax};

use crate::analyze_symbols_usage::SymbolsUsageAnalyzer;
use crate::analyzed_module::AnalyzedModule;
use crate::module_symbols::{merge_iter, Export, Import, ImportedSymbol, ModuleSymbols};

pub fn analyze_file(path: &Path) -> AnalyzedModule<String> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.load_file(path).expect("failed to load test.js");
    let ts_config = TsSyntax {
        tsx: path.ends_with(".tsx"),
        ..TsSyntax::default()
    };

    let mut recovered_errors: Vec<Error> = Vec::new();

    let syntax = Syntax::Typescript(ts_config);

    let module = parse_file_as_module(&fm, syntax, EsVersion::EsNext, None, &mut recovered_errors)
        .unwrap_or_else(|_| panic!("Failed on {path:?}"));

    let mut symbols = analyze_module_symbols(module.clone());
    let symbol_usage_analyze = SymbolsUsageAnalyzer::new(
        symbols
            .imports
            .iter()
            .flat_map(|import| import.symbols.clone())
            .collect::<HashSet<Import>>(),
    );
    let symbols_usage = symbol_usage_analyze.analyze_symbols_usage(module);
    symbols.usages = symbols_usage;

    AnalyzedModule::new(path.to_str().unwrap().to_string(), symbols)
}

pub fn analyze_module_symbols(module: Module) -> ModuleSymbols<String> {
    merge_iter(module.body.into_iter().map(analyze_module_item))
}

fn analyze_module_item(module_item: ModuleItem) -> ModuleSymbols<String> {
    match module_item {
        ModuleItem::Stmt(_) => ModuleSymbols::default(),
        ModuleItem::ModuleDecl(decl) => analyze_module_decl(decl),
    }
}

fn analyze_module_decl(decl: ModuleDecl) -> ModuleSymbols<String> {
    match decl {
        ModuleDecl::Import(decl) => ModuleSymbols::new_imported_symbol(ImportedSymbol {
            from: decl.src.value.to_string(),
            symbols: decl
                .specifiers
                .into_iter()
                .map(analyze_import_specifier)
                .collect(),
        }),
        ModuleDecl::ExportDecl(decl) => analyze_decl(decl.decl),
        ModuleDecl::ExportNamed(decl) => {
            merge_iter(decl.specifiers.iter().map(analyze_export_specifier))
        }
        ModuleDecl::ExportDefaultDecl(_) => ModuleSymbols::new_export(Export::Default),
        ModuleDecl::ExportDefaultExpr(_) => ModuleSymbols::new_export(Export::Default),
        ModuleDecl::ExportAll(decl) => ModuleSymbols::new_all_export(decl.src.value.to_string()),
        ModuleDecl::TsImportEquals(_) => ModuleSymbols::default(), // TODO
        ModuleDecl::TsExportAssignment(_) => unimplemented!(),
        ModuleDecl::TsNamespaceExport(_) => unimplemented!(),
    }
}

fn analyze_export_specifier(decl: &ExportSpecifier) -> ModuleSymbols<String> {
    match decl {
        ExportSpecifier::Named(decl) => ModuleSymbols::new_exported_symbol_str(
            match decl.exported.clone().unwrap_or_else(|| decl.orig.clone()) {
                ModuleExportName::Str(s) => s.value.to_string(),
                ModuleExportName::Ident(s) => s.sym.to_string(),
            },
        ),
        i => todo!("{:#?}", i),
    }
}

fn analyze_import_specifier(decl: ImportSpecifier) -> Import {
    match decl {
        ImportSpecifier::Named(i) => Import::Named(i.local.sym.to_string()),
        ImportSpecifier::Default(i) => Import::Default(i.local.sym.to_string()),
        ImportSpecifier::Namespace(i) => Import::Namespace(i.local.sym.to_string()),
    }
}

fn analyze_decl(decl: Decl) -> ModuleSymbols<String> {
    match decl {
        Decl::Class(class) => {
            ModuleSymbols::new_exported_symbol(class.ident) // .merge(analyze_class(*class.class))
        }
        Decl::Fn(fun) => ModuleSymbols::new_exported_symbol(fun.ident),
        Decl::Var(var) => merge_iter(var.decls.into_iter().map(|decl| analyze_pattern(decl.name))),
        Decl::TsEnum(e) => ModuleSymbols::new_exported_symbol(e.id),
        Decl::TsInterface(i) => ModuleSymbols::new_exported_symbol(i.id),
        Decl::TsTypeAlias(t) => ModuleSymbols::new_exported_symbol(t.id),
        Decl::TsModule(m) => match m.id {
            TsModuleName::Str(_) => ModuleSymbols::default(),
            TsModuleName::Ident(i) => ModuleSymbols::new_exported_symbol(i),
        },
        Decl::Using(_) => todo!("implement Decl::Using"),
    }
}

fn analyze_pattern(pat: Pat) -> ModuleSymbols<String> {
    match pat {
        Pat::Ident(i) => ModuleSymbols::new_exported_symbol(i.id),
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::PathBuf};

    use crate::{
        analyze_file::analyze_file,
        module_symbols::{Export, Import, ImportedSymbol},
    };

    #[test]
    fn namespace_imports() {
        let analyzed_module = analyze_file(&PathBuf::from("./tests/namespace-imports/src/app.ts"));
        assert_eq!(
            analyzed_module.symbols.imports,
            HashSet::from([ImportedSymbol {
                from: "./another".to_string(),
                symbols: vec![Import::Namespace("A".to_string())]
            }])
        );
    }

    #[test]
    fn reexported_symbols() {
        let analyzed_module = analyze_file(&PathBuf::from(
            "./tests/reexported-symbols/src/sub-module/index.ts",
        ));
        assert_eq!(
            analyzed_module.symbols.exports,
            HashSet::from([Export::AllFrom("./module".to_string())])
        );
    }
}
