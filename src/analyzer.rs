use std::path::PathBuf;

use swc_common::sync::Lrc;
use swc_common::SourceMap;
use swc_ecma_ast::{
    BlockStmt, BlockStmtOrExpr, CallExpr, Callee, Class, ClassMember, Decl, Expr, Function, Module,
    ModuleDecl, ModuleItem, Pat, Prop, PropOrSpread, Stmt, TsModuleName,
};
use swc_ecma_ast::{EsVersion, Ident, ImportSpecifier};
use swc_ecma_parser::{error::Error, parse_file_as_module, Syntax, TsConfig};

use crate::module_symbols::ImportedSymbol;
use crate::{
    analyzed_module::AnalyzedModule,
    module_symbols::{merge_iter, ModuleSymbols},
};

pub fn analyze_file(path: PathBuf) -> AnalyzedModule {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm
        .load_file(path.as_path())
        .expect("failed to load test.js");
    let ts_config = TsConfig::default();
    let mut recovered_errors: Vec<Error> = Vec::new();

    let module = parse_file_as_module(
        &fm,
        Syntax::Typescript(ts_config),
        EsVersion::EsNext,
        None,
        &mut recovered_errors,
    )
    .unwrap();

    let symbols = analyze_module_symbols(module);
    AnalyzedModule::new(path, symbols)
}

pub fn analyze_module_symbols(module: Module) -> ModuleSymbols {
    merge_iter(module.body.into_iter().map(analyze_module_item))
}

fn analyze_module_item(module_item: ModuleItem) -> ModuleSymbols {
    match module_item {
        ModuleItem::Stmt(stmt) => analyze_stmt(stmt),
        ModuleItem::ModuleDecl(decl) => analyze_module_decl(decl),
    }
}

fn analyze_module_decl(decl: ModuleDecl) -> ModuleSymbols {
    match decl {
        ModuleDecl::Import(decl) => ModuleSymbols::new_imported_symbol(ImportedSymbol {
            from: decl.src.value.to_string(),
            symbols: decl
                .specifiers
                .into_iter()
                .map(analyze_import_specifier)
                .collect(),
        }),
        ModuleDecl::ExportDecl(decl) => analyze_decl(decl.decl).defined_to_exported(),
        ModuleDecl::ExportNamed(_) => unimplemented!(),
        ModuleDecl::ExportDefaultDecl(_) => unimplemented!(),
        ModuleDecl::ExportDefaultExpr(_) => unimplemented!(),
        ModuleDecl::ExportAll(_) => unimplemented!(),
        ModuleDecl::TsImportEquals(_) => unimplemented!(),
        ModuleDecl::TsExportAssignment(_) => unimplemented!(),
        ModuleDecl::TsNamespaceExport(_) => unimplemented!(),
    }
}

fn analyze_import_specifier(decl: ImportSpecifier) -> Ident {
    match decl {
        ImportSpecifier::Named(i) => i.local,
        _ => unimplemented!()
    }
}

fn analyze_stmt(stmt: Stmt) -> ModuleSymbols {
    match stmt {
        Stmt::Block(stmt) => analyze_block_stmt(stmt),
        Stmt::Empty(_) => ModuleSymbols::default(),
        Stmt::Debugger(_) => ModuleSymbols::default(),
        Stmt::With(stmt) => analyze_stmt(*stmt.body).merge(analyze_expr(*stmt.obj)),
        Stmt::Return(stmt) => analyze_option(|e| analyze_expr(*e), stmt.arg),
        Stmt::Labeled(stmt) => analyze_stmt(*stmt.body),
        Stmt::Break(_) => ModuleSymbols::default(),
        Stmt::Continue(_) => ModuleSymbols::default(),
        Stmt::If(stmt) => analyze_expr(*stmt.test)
            .merge(analyze_stmt(*stmt.cons))
            .merge(analyze_option(|stmt| analyze_stmt(*stmt), stmt.alt)),
        Stmt::Switch(stmt) => {
            analyze_expr(*stmt.discriminant).merge_iter(stmt.cases.into_iter().map(|stmt| {
                merge_iter(stmt.cons.into_iter().map(analyze_stmt))
                    .merge(analyze_option(|expr| analyze_expr(*expr), stmt.test))
            }))
        }
        Stmt::Throw(stmt) => analyze_expr(*stmt.arg),
        Stmt::Try(stmt) => analyze_block_stmt(stmt.block)
            .merge(analyze_option(
                |stmt| analyze_block_stmt(stmt.body),
                stmt.handler,
            ))
            .merge(analyze_option(analyze_block_stmt, stmt.finalizer)),
        Stmt::While(stmt) => analyze_expr(*stmt.test).merge(analyze_stmt(*stmt.body)),
        Stmt::DoWhile(stmt) => analyze_expr(*stmt.test).merge(analyze_stmt(*stmt.body)),
        Stmt::For(stmt) => analyze_option(|s| analyze_expr(*s), stmt.test)
            .merge(analyze_option(|s| analyze_expr(*s), stmt.update))
            .merge(analyze_stmt(*stmt.body)),
        Stmt::ForIn(stmt) => analyze_expr(*stmt.right).merge(analyze_stmt(*stmt.body)),
        Stmt::ForOf(stmt) => analyze_expr(*stmt.right).merge(analyze_stmt(*stmt.body)),
        Stmt::Decl(stmt) => analyze_decl(stmt),
        Stmt::Expr(stmt) => analyze_expr(*stmt.expr),
    }
}

fn analyze_expr(expr: Expr) -> ModuleSymbols {
    match expr {
        Expr::This(_) => ModuleSymbols::default(),
        Expr::Array(expr) => merge_iter(
            expr.elems
                .into_iter()
                .map(|e| analyze_option(|e| analyze_expr(*e.expr), e)),
        ),
        Expr::Object(expr) => merge_iter(expr.props.into_iter().map(analyze_prop_or_spread)),
        Expr::Fn(expr) => analyze_function(*expr.function), // TODO ident
        Expr::Unary(expr) => analyze_expr(*expr.arg),
        Expr::Update(expr) => analyze_expr(*expr.arg),
        Expr::Bin(expr) => analyze_expr(*expr.left).merge(analyze_expr(*expr.right)),
        Expr::Assign(expr) => analyze_expr(*expr.right),
        Expr::Member(expr) => analyze_expr(*expr.obj),
        Expr::SuperProp(_) => ModuleSymbols::default(),
        Expr::Cond(expr) => analyze_expr(*expr.test)
            .merge(analyze_expr(*expr.cons))
            .merge(analyze_expr(*expr.alt)),
        Expr::Call(expr) => analyze_call_expr(expr),
        Expr::New(expr) => analyze_expr(*expr.callee).merge(analyze_option(
            |es| merge_iter(es.into_iter().map(|e| analyze_expr(*e.expr))),
            expr.args,
        )),
        Expr::Seq(expr) => merge_iter(expr.exprs.into_iter().map(|e| analyze_expr(*e))),
        Expr::Ident(i) => ModuleSymbols::new_used_symbol(i),
        Expr::Lit(_) => ModuleSymbols::default(),
        Expr::Tpl(expr) => merge_iter(expr.exprs.into_iter().map(|e| analyze_expr(*e))),
        Expr::TaggedTpl(expr) => merge_iter(expr.tpl.exprs.into_iter().map(|e| analyze_expr(*e)))
            .merge(analyze_expr(*expr.tag)),
        Expr::Arrow(expr) => match *expr.body {
            BlockStmtOrExpr::Expr(expr) => analyze_expr(*expr),
            BlockStmtOrExpr::BlockStmt(stmt) => analyze_block_stmt(stmt),
        },
        Expr::Class(c) => c
            .ident
            .map(ModuleSymbols::new_defined_symbol)
            .unwrap_or_else(ModuleSymbols::default)
            .merge(analyze_class(*c.class)),
        Expr::Yield(expr) => analyze_option(|e| analyze_expr(*e), expr.arg),
        Expr::MetaProp(_) => ModuleSymbols::default(),
        Expr::Await(expr) => analyze_expr(*expr.arg),
        Expr::Paren(expr) => analyze_expr(*expr.expr),
        Expr::JSXMember(_) => unimplemented!(),
        Expr::JSXNamespacedName(_) => unimplemented!(),
        Expr::JSXEmpty(_) => unimplemented!(),
        Expr::JSXElement(_) => unimplemented!(),
        Expr::JSXFragment(_) => unimplemented!(),
        Expr::TsTypeAssertion(expr) => analyze_expr(*expr.expr),
        Expr::TsConstAssertion(expr) => analyze_expr(*expr.expr),
        Expr::TsNonNull(expr) => analyze_expr(*expr.expr),
        Expr::TsAs(expr) => analyze_expr(*expr.expr),
        Expr::TsInstantiation(expr) => analyze_expr(*expr.expr),
        Expr::TsSatisfies(expr) => analyze_expr(*expr.expr),
        Expr::PrivateName(_) => unimplemented!(),
        Expr::OptChain(_) => unimplemented!(),
        Expr::Invalid(_) => unimplemented!(),
    }
}

fn analyze_call_expr(expr: CallExpr) -> ModuleSymbols {
    match expr.callee {
        Callee::Super(_) => ModuleSymbols::default(),
        Callee::Import(_) => ModuleSymbols::default(),
        Callee::Expr(expr) => analyze_expr(*expr),
    }
    .merge(merge_iter(
        expr.args.into_iter().map(|e| analyze_expr(*e.expr)),
    ))
}

fn analyze_class(expr: Class) -> ModuleSymbols {
    merge_iter(expr.body.into_iter().map(|e| match e {
        ClassMember::Constructor(expr) => analyze_option(analyze_block_stmt, expr.body),
        _ => unimplemented!(),
    }))
}

fn analyze_prop_or_spread(expr: PropOrSpread) -> ModuleSymbols {
    match expr {
        PropOrSpread::Prop(p) => match *p {
            Prop::Shorthand(i) => ModuleSymbols::new_used_symbol(i),
            Prop::KeyValue(e) => analyze_expr(*e.value),
            Prop::Assign(e) => analyze_expr(*e.value),
            Prop::Getter(e) => analyze_option(analyze_block_stmt, e.body),
            Prop::Setter(e) => analyze_option(analyze_block_stmt, e.body),
            Prop::Method(e) => analyze_function(*e.function), // TODO maybe analyze propName
        },
        PropOrSpread::Spread(s) => analyze_expr(*s.expr),
    }
}

fn analyze_function(_fun: Function) -> ModuleSymbols {
    unimplemented!()
}

fn analyze_option<F, T>(fun: F, value: Option<T>) -> ModuleSymbols
where
    F: FnOnce(T) -> ModuleSymbols,
{
    value.map(fun).unwrap_or_else(ModuleSymbols::default)
}

fn analyze_block_stmt(stmt: BlockStmt) -> ModuleSymbols {
    merge_iter(stmt.stmts.into_iter().map(analyze_stmt))
}

fn analyze_decl(decl: Decl) -> ModuleSymbols {
    match decl {
        Decl::Class(class) => {
            ModuleSymbols::new_defined_symbol(class.ident).merge(analyze_class(*class.class))
        }
        Decl::Fn(fun) => ModuleSymbols::new_defined_symbol(fun.ident),
        Decl::Var(var) => merge_iter(var.decls.into_iter().map(|decl| analyze_pattern(decl.name))),
        Decl::TsEnum(e) => ModuleSymbols::new_defined_symbol(e.id),
        Decl::TsInterface(i) => ModuleSymbols::new_defined_symbol(i.id),
        Decl::TsTypeAlias(t) => ModuleSymbols::new_defined_symbol(t.id),
        Decl::TsModule(m) => match m.id {
            TsModuleName::Str(_) => ModuleSymbols::default(),
            TsModuleName::Ident(i) => ModuleSymbols::new_defined_symbol(i),
        },
    }
}

fn analyze_pattern(pat: Pat) -> ModuleSymbols {
    match pat {
        Pat::Ident(i) => ModuleSymbols::new_defined_symbol(i.id),
        _ => unimplemented!(),
    }
}
