use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use swc_core::quote;
use swc_core::{
    common::{Span, Spanned, SyntaxContext, util::take::Take, DUMMY_SP},
    ecma::{
        ast::*,
        utils::{quote_ident, IsDirective},
        visit::{VisitMut, VisitMutWith},
    },
};

use crate::source_coverage::{Range, SourceCoverage};

/// 学 old：创建 cov_xxx().s[id]++ 表达式
fn create_increase_counter_expr(
    id: u32,
    cov_fn_ident: &Ident,
) -> Expr {
    let ident_s = Ident::new("s".into(), DUMMY_SP, Default::default());
    let call = CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Ident(cov_fn_ident.clone()))),
        args: vec![],
        type_args: None,
        ..Default::default()
    };
    let c = MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Call(call)),
        prop: MemberProp::Ident(ident_s.into()),
    };
    let expr = MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Member(c)),
        prop: MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: id as f64,
                raw: None,
            }))),
        }),
    };
    Expr::Update(UpdateExpr {
        span: DUMMY_SP,
        op: UpdateOp::PlusPlus,
        prefix: false,
        arg: Box::new(Expr::Member(expr)),
    })
}

/// 创建 Istanbul Range 的 AST：{ start: { line, column }, end: { line, column } }
fn create_range_object_lit(range: &Range) -> Expr {
    let ident_start = Ident::new("start".into(), DUMMY_SP, Default::default());
    let ident_end = Ident::new("end".into(), DUMMY_SP, Default::default());
    let ident_line = Ident::new("line".into(), DUMMY_SP, Default::default());
    let ident_column = Ident::new("column".into(), DUMMY_SP, Default::default());
    Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(ident_start.clone().into()),
                value: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(ident_line.clone().into()),
                            value: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: range.start.line as f64,
                                raw: None,
                            }))),
                        }))),
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(ident_column.clone().into()),
                            value: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: range.start.column as f64,
                                raw: None,
                            }))),
                        }))),
                    ],
                })),
            }))),
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(ident_end.into()),
                value: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(Ident::new("line".into(), DUMMY_SP, Default::default()).into()),
                            value: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: range.end.line as f64,
                                raw: None,
                            }))),
                        }))),
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(Ident::new("column".into(), DUMMY_SP, Default::default()).into()),
                            value: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: range.end.column as f64,
                                raw: None,
                            }))),
                        }))),
                    ],
                })),
            }))),
        ],
    })
}

/// 在每个文件顶部注入全局 __coverage__ 初始化
/// 学 old：收集 statements，生成 statementMap、s，并注入 cov().s[id]++
pub struct CoverageVisitor {
    filename: String,
    cov: RefCell<SourceCoverage>,
    cov_fn_ident: Ident,
    get_range: Box<dyn Fn(&Span) -> Range + Send + Sync>,
}

impl CoverageVisitor {
    pub fn new(filename: String, get_range: Box<dyn Fn(&Span) -> Range + Send + Sync>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        filename.hash(&mut hasher);
        let cov_fn_name = format!("cov_{}", hasher.finish());
        Self {
            cov_fn_ident: Ident::new(cov_fn_name.into(), DUMMY_SP, Default::default()),
            filename,
            cov: RefCell::new(SourceCoverage::new()),
            get_range,
        }
    }

    fn get_range(&self, span: &Span) -> Range {
        (self.get_range)(span)
    }

    /// var ident = value;
    fn create_assignment_stmt(ident: &Ident, value: Expr) -> Stmt {
        Stmt::Decl(Decl::Var(Box::new(VarDecl {
            kind: VarDeclKind::Var,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Assign(AssignPat {
                    span: DUMMY_SP,
                    left: Box::new(Pat::Ident(BindingIdent::from(ident.clone()))),
                    right: Box::new(value),
                }),
                init: None,
                definite: false,
            }],
            ..VarDecl::dummy()
        })))
    }

    /// 创建全局 coverage 初始化语句（学 old）
    fn create_window_coverage_init_stmts(&self) -> Vec<Stmt> {
        let cov = self.cov.borrow();
        let path_expr = Expr::Lit(Lit::Str(Str {
            value: self.filename.clone().into(),
            raw: Some(format!(r#""{}""#, self.filename).into()),
            span: DUMMY_SP,
        }));

        let ident_global = Ident::new("global".into(), DUMMY_SP, Default::default());

        // var global = new Function("return this")();
        let global_stmt = {
            let fn_ctor = quote_ident!(Default::default(), "((function(){}).constructor)");
            let expr = Expr::New(NewExpr {
                callee: Box::new(Expr::Ident(fn_ctor)),
                args: Some(vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        value: "return this".into(),
                        ..Str::dummy()
                    }))),
                }]),
                ..NewExpr::dummy()
            });
            Self::create_assignment_stmt(
                &ident_global,
                Expr::Call(CallExpr {
                    callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
                        span: DUMMY_SP,
                        expr: Box::new(expr),
                    }))),
                    ..CallExpr::dummy()
                }),
            )
        };

        // global.__coverage__ = global.__coverage__ || {};
        let init_coverage = quote!(
            "global.__coverage__ = global.__coverage__ || {}" as Stmt
        );

        // statementMap: { "0": { start, end }, ... }
        let statement_map_props: Vec<PropOrSpread> = cov
            .statement_map
            .iter()
            .map(|(k, v)| {
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Str(Str {
                        value: k.to_string().into(),
                        ..Str::dummy()
                    }),
                    value: Box::new(create_range_object_lit(v)),
                })))
            })
            .collect();

        // s: { "0": 0, "1": 0, ... }
        let s_props: Vec<PropOrSpread> = cov
            .s
            .iter()
            .map(|(k, v)| {
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Str(Str {
                        value: k.to_string().into(),
                        ..Str::dummy()
                    }),
                    value: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: *v as f64,
                        raw: None,
                    }))),
                })))
            })
            .collect();

        // coverage 数据对象
        let coverage_data = Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("path".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(path_expr.clone()),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("statementMap".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: statement_map_props,
                    })),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("fnMap".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![],
                    })),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("branchMap".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![],
                    })),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("s".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: s_props,
                    })),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("f".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![],
                    })),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(Ident::new("b".into(), DUMMY_SP, Default::default()).into()),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![],
                    })),
                }))),
            ],
        });

        // global.__coverage__[path] = coverage_data
        let global_cov_member = MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(ident_global.clone())),
                prop: MemberProp::Ident(Ident::new("__coverage__".into(), DUMMY_SP, Default::default()).into()),
            })),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(path_expr),
            }),
        };
        let assign_coverage = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Assign(AssignExpr {
                span: DUMMY_SP,
                op: AssignOp::Assign,
                left: AssignTarget::Simple(SimpleAssignTarget::Member(global_cov_member)),
                right: Box::new(coverage_data),
            })),
        });

        // cov_xxx = function() { return global.__coverage__[path]; }
        let path_expr2 = Expr::Lit(Lit::Str(Str {
            value: self.filename.clone().into(),
            raw: Some(format!(r#""{}""#, self.filename).into()),
            span: DUMMY_SP,
        }));
        let cov_fn_assign = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Assign(AssignExpr {
                span: DUMMY_SP,
                op: AssignOp::Assign,
                left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent::from(
                    self.cov_fn_ident.clone(),
                ))),
                right: Box::new(Expr::Fn(FnExpr {
                    ident: None,
                    function: Box::new(Function {
                        params: vec![],
                        decorators: vec![],
                        span: DUMMY_SP,
                        body: Some(BlockStmt {
                            span: DUMMY_SP,
                            stmts: vec![Stmt::Return(ReturnStmt {
                                span: DUMMY_SP,
                                arg: Some(Box::new(Expr::Member(MemberExpr {
                                    span: DUMMY_SP,
                                    obj: Box::new(Expr::Member(MemberExpr {
                                        span: DUMMY_SP,
                                        obj: Box::new(Expr::Ident(ident_global)),
                                        prop: MemberProp::Ident(Ident::new("__coverage__".into(), DUMMY_SP, Default::default()).into()),
                                    })),
                                    prop: MemberProp::Computed(ComputedPropName {
                                        span: DUMMY_SP,
                                        expr: Box::new(path_expr2),
                                    }),
                                }))),
                            })],
                            ..BlockStmt::dummy()
                        }),
                        is_generator: false,
                        is_async: false,
                        type_params: None,
                        return_type: None,
                        ctxt: SyntaxContext::empty(),
                    }),
                })),
                ..AssignExpr::dummy()
            })),
        });

        vec![global_stmt, init_coverage, assign_coverage, cov_fn_assign]
    }

    /// 在 statement 前插入 counter
    fn mark_prepend_stmt_counter(&self, span: &Span) -> Stmt {
        let range = self.get_range(span);
        let id = self.cov.borrow_mut().new_statement(&range);
        println!("      创建 counter: id={}, range={:?}", id, range);
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(create_increase_counter_expr(id, &self.cov_fn_ident)),
        })
    }
}

impl VisitMut for CoverageVisitor {
    /// 学 old visit_mut_module_items：对顶层 Stmt 注入 counter
    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        println!("=== visit_mut_module_items: 处理 {} 个模块项 ===", items.len());
        let mut new_items = Vec::new();
        for (idx, mut item) in items.drain(..).enumerate() {
            if let ModuleItem::Stmt(ref stmt) = &item {
                let span = stmt.span();
                // 跳过 DUMMY_SP 的语句、directive 和函数声明
                // 函数声明不应该被当作语句插桩，只有函数体内的语句才需要插桩
                let should_skip = span == DUMMY_SP 
                    || stmt.directive_continue()
                    || matches!(stmt, Stmt::Decl(Decl::Fn(_)));
                
                if !should_skip {
                    println!("  [{}] 模块语句: {:?}", idx, stmt);
                    let range = self.get_range(&span);
                    println!("    -> 注入 counter, range: {:?}", range);
                    let counter = self.mark_prepend_stmt_counter(&span);
                    new_items.push(ModuleItem::Stmt(counter));
                } else {
                    println!("  [{}] 跳过语句（DUMMY_SP、directive 或函数声明）", idx);
                }
            }
            item.visit_mut_children_with(self);
            new_items.push(item);
        }
        *items = new_items;
    }

    fn visit_mut_program(&mut self, program: &mut Program) {
        println!("=== visit_mut_program: 开始处理程序 ===");
        program.visit_mut_children_with(self);

        let stmts = self.create_window_coverage_init_stmts();
        println!("=== 创建了 {} 个初始化语句 ===", stmts.len());

        match program {
            Program::Module(m) => {
                println!("  -> 插入到 Module 顶部");
                for stmt in stmts.into_iter().rev() {
                    m.body.insert(0, ModuleItem::Stmt(stmt));
                }
            }
            Program::Script(s) => {
                println!("  -> 插入到 Script 顶部");
                for stmt in stmts.into_iter().rev() {
                    s.body.insert(0, stmt);
                }
            }
            _ => {}
        }
        println!("=== visit_mut_program: 处理完成 ===");
    }

    /// 学 old visit_mut_script：对 Script body 的 stmt 注入 counter
    fn visit_mut_script(&mut self, script: &mut Script) {
        println!("=== visit_mut_script: 处理 {} 个脚本语句 ===", script.body.len());
        let mut new_stmts = Vec::new();
        for (idx, mut stmt) in script.body.drain(..).enumerate() {
            let span = stmt.span();
            // 跳过 DUMMY_SP 的语句、directive 和函数声明
            let should_skip = span == DUMMY_SP 
                || stmt.directive_continue()
                || matches!(stmt, Stmt::Decl(Decl::Fn(_)));
            
            if !should_skip {
                println!("  [{}] 脚本语句: {:?}", idx, stmt);
                let range = self.get_range(&span);
                println!("    -> 注入 counter, range: {:?}", range);
                let counter = self.mark_prepend_stmt_counter(&span);
                new_stmts.push(counter);
            } else {
                println!("  [{}] 跳过语句（DUMMY_SP、directive 或函数声明）", idx);
            }
            stmt.visit_mut_children_with(self);
            new_stmts.push(stmt);
        }
        script.body = new_stmts;
    }

    /// 学 old visit_mut_stmts：对 BlockStmt 内的 stmt 注入 counter
    fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        println!("=== visit_mut_stmts: 处理 {} 个块语句 ===", stmts.len());
        let mut new_stmts = Vec::new();
        for (idx, mut stmt) in stmts.drain(..).enumerate() {
            let span = stmt.span();
            // 跳过 DUMMY_SP 的语句（插桩生成的语句）
            if span != DUMMY_SP {
                println!("  [{}] 块语句: {:?}", idx, stmt);
                let range = self.get_range(&span);
                println!("    -> 注入 counter, range: {:?}", range);
                let counter = self.mark_prepend_stmt_counter(&span);
                new_stmts.push(counter);
            } else {
                println!("  [{}] 跳过 DUMMY_SP 语句（插桩生成）", idx);
            }
            stmt.visit_mut_children_with(self);
            new_stmts.push(stmt);
        }
        *stmts = new_stmts;
    }
}

/// 创建 coverage instrumentation visitor
/// get_range: 将 Span 转为 Istanbul Range，无 source_map 时传入 |_| Range::default()
pub fn create_coverage_instrumentation_visitor<F>(filename: &str, get_range: F) -> CoverageVisitor
where
    F: Fn(&Span) -> Range + Send + Sync + 'static,
{
    CoverageVisitor::new(filename.to_string(), Box::new(get_range))
}
