use std::collections::HashSet;

use swc_ecma_ast::{
    BlockStmt, BlockStmtOrExpr, CallExpr, Callee, Class, ClassMember, Decl, DefaultDecl, Expr,
    Function, MemberExpr, MemberProp, Module, ModuleDecl, ModuleItem, Prop, PropOrSpread, Stmt,
    TsInterfaceDecl,
};

use crate::module_symbols::{Import, Usage};

pub struct SymbolsUsageAnalyzer {
    imports: HashSet<Import>,
}

impl SymbolsUsageAnalyzer {
    pub fn new(imports: HashSet<Import>) -> Self {
        Self { imports }
    }

    pub fn analyze_symbols_usage(&self, module: Module) -> HashSet<Usage> {
        module
            .body
            .into_iter()
            .map(|u| self.analyze_module_item(u))
            .reduce(|acc, cur| acc.union(&cur).cloned().collect::<HashSet<_>>())
            .unwrap_or(Default::default())
    }

    fn analyze_module_item(&self, module_item: ModuleItem) -> HashSet<Usage> {
        match module_item {
            ModuleItem::Stmt(stmt) => self.analyze_stmt(stmt),
            ModuleItem::ModuleDecl(decl) => self.analyze_module_decl(decl),
        }
    }

    fn analyze_module_decl(&self, decl: ModuleDecl) -> HashSet<Usage> {
        match decl {
            ModuleDecl::Import(_) => HashSet::default(),
            ModuleDecl::ExportDecl(decl) => self.analyze_decl(decl.decl),
            ModuleDecl::ExportNamed(_) => Default::default(),
            ModuleDecl::ExportDefaultDecl(decl) => match decl.decl {
                DefaultDecl::Fn(decl) => self.analyze_function(*decl.function),
                DefaultDecl::Class(decl) => self.analyze_class(*decl.class),
                DefaultDecl::TsInterfaceDecl(decl) => self.analyze_tsinterface(*decl),
            },
            ModuleDecl::ExportDefaultExpr(decl) => self.analyze_expr(*decl.expr),
            ModuleDecl::ExportAll(_) => Default::default(),
            ModuleDecl::TsImportEquals(_) => Default::default(),
            ModuleDecl::TsExportAssignment(_) => unimplemented!(),
            ModuleDecl::TsNamespaceExport(_) => unimplemented!(),
        }
    }

    fn analyze_tsinterface(&self, decl: TsInterfaceDecl) -> HashSet<Usage> {
        todo!("{decl:?}")
    }

    fn analyze_stmt(&self, stmt: Stmt) -> HashSet<Usage> {
        match stmt {
            Stmt::Block(stmt) => self.analyze_block_stmt(stmt),
            Stmt::Empty(_) => HashSet::default(),
            Stmt::Debugger(_) => HashSet::default(),
            Stmt::With(stmt) => {
                merge_usages(self.analyze_stmt(*stmt.body), self.analyze_expr(*stmt.obj))
            }
            Stmt::Return(stmt) => self.analyze_option(|e| self.analyze_expr(*e), stmt.arg),
            Stmt::Labeled(stmt) => self.analyze_stmt(*stmt.body),
            Stmt::Break(_) => HashSet::default(),
            Stmt::Continue(_) => HashSet::default(),
            Stmt::If(stmt) => merge_usages(
                merge_usages(self.analyze_expr(*stmt.test), self.analyze_stmt(*stmt.cons)),
                self.analyze_option(|stmt| self.analyze_stmt(*stmt), stmt.alt),
            ),
            Stmt::Switch(stmt) => merge_usages(
                self.analyze_expr(*stmt.discriminant),
                merge_usages_iter(stmt.cases.into_iter().map(|stmt| {
                    merge_usages(
                        merge_usages_iter(stmt.cons.into_iter().map(|e| self.analyze_stmt(e))),
                        self.analyze_option(|expr| self.analyze_expr(*expr), stmt.test),
                    )
                })),
            ),
            Stmt::Throw(stmt) => self.analyze_expr(*stmt.arg),
            Stmt::Try(stmt) => merge_usages(
                merge_usages(
                    self.analyze_block_stmt(stmt.block),
                    self.analyze_option(|stmt| self.analyze_block_stmt(stmt.body), stmt.handler),
                ),
                self.analyze_option(|b| self.analyze_block_stmt(b), stmt.finalizer),
            ),
            Stmt::While(stmt) => {
                merge_usages(self.analyze_expr(*stmt.test), self.analyze_stmt(*stmt.body))
            }
            Stmt::DoWhile(stmt) => {
                merge_usages(self.analyze_expr(*stmt.test), self.analyze_stmt(*stmt.body))
            }
            Stmt::For(stmt) => merge_usages(
                merge_usages(
                    self.analyze_option(|s| self.analyze_expr(*s), stmt.test),
                    self.analyze_option(|s| self.analyze_expr(*s), stmt.update),
                ),
                self.analyze_stmt(*stmt.body),
            ),
            Stmt::ForIn(stmt) => merge_usages(
                self.analyze_expr(*stmt.right),
                self.analyze_stmt(*stmt.body),
            ),
            Stmt::ForOf(stmt) => merge_usages(
                self.analyze_expr(*stmt.right),
                self.analyze_stmt(*stmt.body),
            ),
            Stmt::Decl(stmt) => self.analyze_decl(stmt),
            Stmt::Expr(stmt) => self.analyze_expr(*stmt.expr),
        }
    }

    fn analyze_expr(&self, expr: Expr) -> HashSet<Usage> {
        match expr {
            Expr::This(_) => HashSet::default(),
            Expr::Array(expr) => merge_usages_iter(
                expr.elems
                    .into_iter()
                    .map(|e| self.analyze_option(|e| self.analyze_expr(*e.expr), e)),
            ),
            Expr::Object(expr) => merge_usages_iter(
                expr.props
                    .into_iter()
                    .map(|e| self.analyze_prop_or_spread(e)),
            ),
            Expr::Fn(expr) => self.analyze_function(*expr.function), // TODO ident
            Expr::Unary(expr) => self.analyze_expr(*expr.arg),
            Expr::Update(expr) => self.analyze_expr(*expr.arg),
            Expr::Bin(expr) => merge_usages(
                self.analyze_expr(*expr.left),
                self.analyze_expr(*expr.right),
            ),
            Expr::Assign(expr) => self.analyze_expr(*expr.right),
            Expr::Member(expr) => self.analyze_member_expr(expr),
            Expr::SuperProp(_) => HashSet::default(),
            Expr::Cond(expr) => merge_usages(
                merge_usages(self.analyze_expr(*expr.test), self.analyze_expr(*expr.cons)),
                self.analyze_expr(*expr.alt),
            ),
            Expr::Call(expr) => self.analyze_call_expr(expr),
            Expr::New(expr) => merge_usages(
                self.analyze_expr(*expr.callee),
                self.analyze_option(
                    |es| merge_usages_iter(es.into_iter().map(|e| self.analyze_expr(*e.expr))),
                    expr.args,
                ),
            ),
            Expr::Seq(expr) => {
                merge_usages_iter(expr.exprs.into_iter().map(|e| self.analyze_expr(*e)))
            }
            Expr::Ident(_) => HashSet::default(),
            Expr::Lit(_) => HashSet::default(),
            Expr::Tpl(expr) => {
                merge_usages_iter(expr.exprs.into_iter().map(|e| self.analyze_expr(*e)))
            }
            Expr::TaggedTpl(expr) => merge_usages(
                merge_usages_iter(expr.tpl.exprs.into_iter().map(|e| self.analyze_expr(*e))),
                self.analyze_expr(*expr.tag),
            ),
            Expr::Arrow(expr) => match *expr.body {
                BlockStmtOrExpr::Expr(expr) => self.analyze_expr(*expr),
                BlockStmtOrExpr::BlockStmt(stmt) => self.analyze_block_stmt(stmt),
            },
            Expr::Class(c) => self.analyze_class(*c.class),
            Expr::Yield(expr) => self.analyze_option(|e| self.analyze_expr(*e), expr.arg),
            Expr::MetaProp(_) => HashSet::default(),
            Expr::Await(expr) => self.analyze_expr(*expr.arg),
            Expr::Paren(expr) => self.analyze_expr(*expr.expr),
            Expr::JSXMember(_) => unimplemented!(),
            Expr::JSXNamespacedName(_) => unimplemented!(),
            Expr::JSXEmpty(_) => unimplemented!(),
            Expr::JSXElement(_) => Default::default(), // TODO
            Expr::JSXFragment(_) => Default::default(), // TODO
            Expr::TsTypeAssertion(expr) => self.analyze_expr(*expr.expr),
            Expr::TsConstAssertion(expr) => self.analyze_expr(*expr.expr),
            Expr::TsNonNull(expr) => self.analyze_expr(*expr.expr),
            Expr::TsAs(expr) => self.analyze_expr(*expr.expr),
            Expr::TsInstantiation(expr) => self.analyze_expr(*expr.expr),
            Expr::TsSatisfies(expr) => self.analyze_expr(*expr.expr),
            Expr::PrivateName(_) => unimplemented!(),
            Expr::OptChain(_) => Default::default(), // TODO
            Expr::Invalid(_) => unimplemented!(),
        }
    }

    fn analyze_member_expr(&self, expr: MemberExpr) -> HashSet<Usage> {
        match (*expr.obj, expr.prop) {
            (Expr::Ident(alias), MemberProp::Ident(symbol)) => {
                let alias = alias.sym.to_string();
                let symbol = symbol.sym.to_string();

                if self.imports.contains(&Import::Namespace(alias.to_owned())) {
                    return HashSet::from([Usage::Namespace(symbol, alias)]);
                }
            }
            (obj, _) => return self.analyze_expr(obj),
        }

        Default::default()
    }

    fn analyze_call_expr(&self, expr: CallExpr) -> HashSet<Usage> {
        let args = merge_usages_iter(expr.args.into_iter().map(|e| self.analyze_expr(*e.expr)));
        let callee = match expr.callee {
            Callee::Super(_) => HashSet::default(),
            Callee::Import(_) => HashSet::default(),
            Callee::Expr(expr) => self.analyze_expr(*expr),
        };
        merge_usages(args, callee)
    }

    fn analyze_class(&self, expr: Class) -> HashSet<Usage> {
        merge_usages_iter(expr.body.into_iter().map(|e| match e {
            ClassMember::Constructor(expr) => {
                self.analyze_option(|e| self.analyze_block_stmt(e), expr.body)
            }
            ClassMember::Method(expr) => self.analyze_function(*expr.function),
            i => todo!("{i:#?}"),
        }))
    }

    fn analyze_prop_or_spread(&self, expr: PropOrSpread) -> HashSet<Usage> {
        match expr {
            PropOrSpread::Prop(p) => match *p {
                Prop::Shorthand(_) => HashSet::default(),
                Prop::KeyValue(e) => self.analyze_expr(*e.value),
                Prop::Assign(e) => self.analyze_expr(*e.value),
                Prop::Getter(e) => self.analyze_option(|e| self.analyze_block_stmt(e), e.body),
                Prop::Setter(e) => self.analyze_option(|e| self.analyze_block_stmt(e), e.body),
                Prop::Method(e) => self.analyze_function(*e.function), // TODO maybe analyze propName
            },
            PropOrSpread::Spread(s) => self.analyze_expr(*s.expr),
        }
    }

    fn analyze_function(&self, fun: Function) -> HashSet<Usage> {
        fun.body
            .map(|b| self.analyze_block_stmt(b))
            .unwrap_or(Default::default())
    }

    fn analyze_option<F, T>(&self, fun: F, value: Option<T>) -> HashSet<Usage>
    where
        F: FnOnce(T) -> HashSet<Usage>,
    {
        value.map(fun).unwrap_or_else(HashSet::default)
    }

    fn analyze_block_stmt(&self, stmt: BlockStmt) -> HashSet<Usage> {
        merge_usages_iter(stmt.stmts.into_iter().map(|e| self.analyze_stmt(e)))
    }

    fn analyze_decl(&self, decl: Decl) -> HashSet<Usage> {
        match decl {
            Decl::Class(class) => self.analyze_class(*class.class),
            Decl::Fn(fun) => self.analyze_function(*fun.function),
            Decl::Var(var) => merge_usages_iter(
                var.decls
                    .into_iter()
                    .filter_map(|decl| decl.init.map(|i| self.analyze_expr(*i))),
            ),
            Decl::TsEnum(_) => HashSet::default(),      // TODO
            Decl::TsInterface(_) => HashSet::default(), // TODO
            Decl::TsTypeAlias(_) => HashSet::default(), // TODO
            Decl::TsModule(_) => HashSet::default(),    // TODO
            Decl::Using(_) => HashSet::default(),       // TODO
        }
    }
}

fn merge_usages_iter<Iter: IntoIterator<Item = HashSet<Usage>>>(iter: Iter) -> HashSet<Usage> {
    iter.into_iter()
        .reduce(|acc, cur| acc.union(&cur).cloned().collect::<HashSet<_>>())
        .unwrap_or(Default::default())
}

fn merge_usages(u1: HashSet<Usage>, u2: HashSet<Usage>) -> HashSet<Usage> {
    u1.union(&u2).cloned().collect() // TODO should be possible to drain instead of copy
}
