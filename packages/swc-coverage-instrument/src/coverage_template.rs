use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::{ast::*, utils::quote_ident},
};

use crate::source_coverage::{Range, SourceCoverage};

/// 创建 Range 对象字面量: { start: { line, column }, end: { line, column } }
fn create_range_object_lit(range: &Range) -> Expr {
    Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(Ident::new("start".into(), DUMMY_SP, Default::default()).into()),
                value: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(Ident::new("line".into(), DUMMY_SP, Default::default()).into()),
                            value: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: range.start.line as f64,
                                raw: None,
                            }))),
                        }))),
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(Ident::new("column".into(), DUMMY_SP, Default::default()).into()),
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
                key: PropName::Ident(Ident::new("end".into(), DUMMY_SP, Default::default()).into()),
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

/// 创建覆盖率数据对象
fn create_coverage_data_object(filename: &str, cov: &SourceCoverage) -> Expr {
    // statementMap: { "0": { start, end }, ... }
    let statement_map_props: Vec<PropOrSpread> = cov
        .statement_map
        .iter()
        .map(|(k, v)| {
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Str(Str {
                    value: k.to_string().into(),
                    span: DUMMY_SP,
                    raw: None,
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
                    span: DUMMY_SP,
                    raw: None,
                }),
                value: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: *v as f64,
                    raw: None,
                }))),
            })))
        })
        .collect();

    Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(Ident::new("path".into(), DUMMY_SP, Default::default()).into()),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    value: filename.into(),
                    span: DUMMY_SP,
                    raw: Some(format!(r#""{}""#, filename).into()),
                }))),
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
    })
}

/// 创建 var ident = value; 语句
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

/// 计算覆盖率数据的 hash
fn compute_hash(_filename: &str, cov: &SourceCoverage) -> String {
    let coverage_str = format!("{:?}", cov);
    let mut hasher = DefaultHasher::new();
    coverage_str.hash(&mut hasher);
    hasher.finish().to_string()
}

/// 创建覆盖率函数声明
/// 生成类似这样的代码：
/// ```javascript
/// function cov_xxx() {
///   var path = "src/file.js";
///   var hash = "...";
///   var global = new Function("return this")();
///   var gcv = "__coverage__";
///   var coverageData = { ... };
///   var coverage = global[gcv] || (global[gcv] = {});
///   if (!coverage[path] || coverage[path].hash !== hash) {
///     coverage[path] = coverageData;
///   }
///   var actualCoverage = coverage[path];
///   return actualCoverage;
/// }
/// ```
fn create_coverage_fn_decl(
    filename: &str,
    cov_fn_ident: &Ident,
    cov: &SourceCoverage,
) -> Stmt {
    let mut stmts = vec![];

    // var path = "src/file.js";
    let ident_path = Ident::new("path".into(), DUMMY_SP, Default::default());
    stmts.push(create_assignment_stmt(
        &ident_path,
        Expr::Lit(Lit::Str(Str {
            value: filename.into(),
            span: DUMMY_SP,
            raw: Some(format!(r#""{}""#, filename).into()),
        })),
    ));

    // var hash = "...";
    let hash = compute_hash(filename, cov);
    let ident_hash = Ident::new("hash".into(), DUMMY_SP, Default::default());
    stmts.push(create_assignment_stmt(
        &ident_hash,
        Expr::Lit(Lit::Str(Str {
            value: hash.clone().into(),
            span: DUMMY_SP,
            raw: Some(format!(r#""{}""#, hash).into()),
        })),
    ));

    // var global = new Function("return this")();
    let ident_global = Ident::new("global".into(), DUMMY_SP, Default::default());
    let fn_ctor = quote_ident!(Default::default(), "Function");
    stmts.push(create_assignment_stmt(
        &ident_global,
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(Expr::New(NewExpr {
                span: DUMMY_SP,
                ctxt: Default::default(),
                callee: Box::new(Expr::Ident(fn_ctor)),
                args: Some(vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        value: "return this".into(),
                        span: DUMMY_SP,
                        raw: Some(r#""return this""#.into()),
                    }))),
                }]),
                type_args: None,
            }))),
            args: vec![],
            type_args: None,
        }),
    ));

    // var gcv = "__coverage__";
    let ident_gcv = Ident::new("gcv".into(), DUMMY_SP, Default::default());
    stmts.push(create_assignment_stmt(
        &ident_gcv,
        Expr::Lit(Lit::Str(Str {
            value: "__coverage__".into(),
            span: DUMMY_SP,
            raw: Some(r#""__coverage__""#.into()),
        })),
    ));

    // var coverageData = { ... };
    let ident_coverage_data = Ident::new("coverageData".into(), DUMMY_SP, Default::default());
    stmts.push(create_assignment_stmt(
        &ident_coverage_data,
        create_coverage_data_object(filename, cov),
    ));

    // var coverage = global[gcv] || (global[gcv] = {});
    let ident_coverage = Ident::new("coverage".into(), DUMMY_SP, Default::default());
    stmts.push(create_assignment_stmt(
        &ident_coverage,
        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::LogicalOr,
            left: Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(ident_global.clone())),
                prop: MemberProp::Computed(ComputedPropName {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Ident(ident_gcv.clone())),
                }),
            })),
            right: Box::new(Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(Expr::Assign(AssignExpr {
                    span: DUMMY_SP,
                    op: AssignOp::Assign,
                    left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::Ident(ident_global.clone())),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Ident(ident_gcv.clone())),
                        }),
                    })),
                    right: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![],
                    })),
                })),
            })),
        }),
    ));

    // if (!coverage[path] || coverage[path].hash !== hash) {
    //   coverage[path] = coverageData;
    // }
    stmts.push(Stmt::If(IfStmt {
        span: DUMMY_SP,
        test: Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::LogicalOr,
            left: Box::new(Expr::Unary(UnaryExpr {
                span: DUMMY_SP,
                op: UnaryOp::Bang,
                arg: Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(Expr::Ident(ident_coverage.clone())),
                    prop: MemberProp::Computed(ComputedPropName {
                        span: DUMMY_SP,
                        expr: Box::new(Expr::Ident(ident_path.clone())),
                    }),
                })),
            })),
            right: Box::new(Expr::Bin(BinExpr {
                span: DUMMY_SP,
                op: BinaryOp::NotEqEq,
                left: Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::Ident(ident_coverage.clone())),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Ident(ident_path.clone())),
                        }),
                    })),
                    prop: MemberProp::Ident(ident_hash.clone().into()),
                })),
                right: Box::new(Expr::Ident(ident_hash.clone())),
            })),
        })),
        cons: Box::new(Stmt::Block(BlockStmt {
            span: DUMMY_SP,
            stmts: vec![Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(Expr::Assign(AssignExpr {
                    span: DUMMY_SP,
                    op: AssignOp::Assign,
                    left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::Ident(ident_coverage.clone())),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Ident(ident_path.clone())),
                        }),
                    })),
                    right: Box::new(Expr::Ident(ident_coverage_data.clone())),
                })),
            })],
            ..BlockStmt::dummy()
        })),
        alt: None,
    }));

    // var actualCoverage = coverage[path];
    let ident_actual_coverage = Ident::new("actualCoverage".into(), DUMMY_SP, Default::default());
    stmts.push(create_assignment_stmt(
        &ident_actual_coverage,
        Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(ident_coverage.clone())),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Ident(ident_path.clone())),
            }),
        }),
    ));

    // {
    //   cov_xxx = function() { return actualCoverage; };
    // }
    stmts.push(Stmt::Block(BlockStmt {
        span: DUMMY_SP,
        stmts: vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Assign(AssignExpr {
                span: DUMMY_SP,
                op: AssignOp::Assign,
                left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent::from(
                    cov_fn_ident.clone(),
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
                                arg: Some(Box::new(Expr::Ident(ident_actual_coverage.clone()))),
                            })],
                            ..BlockStmt::dummy()
                        }),
                        is_generator: false,
                        is_async: false,
                        type_params: None,
                        return_type: None,
                        ctxt: Default::default(),
                    }),
                })),
            })),
        })],
        ..BlockStmt::dummy()
    }));

    // return actualCoverage;
    stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(Box::new(Expr::Ident(ident_actual_coverage))),
    }));

    // function cov_xxx() { ... }
    Stmt::Decl(Decl::Fn(FnDecl {
        ident: cov_fn_ident.clone(),
        declare: false,
        function: Box::new(Function {
            params: vec![],
            decorators: vec![],
            span: DUMMY_SP,
            body: Some(BlockStmt {
                span: DUMMY_SP,
                stmts,
                ..BlockStmt::dummy()
            }),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: None,
            ctxt: Default::default(),
        }),
    }))
}

/// 创建覆盖率初始化语句
/// 返回: [函数声明, 调用语句]
pub fn create_coverage_init_stmts(
    filename: &str,
    cov_fn_ident: &Ident,
    cov: &SourceCoverage,
) -> Vec<Stmt> {
    vec![
        // function cov_xxx() { ... }
        create_coverage_fn_decl(filename, cov_fn_ident, cov),
        // cov_xxx();
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                ctxt: Default::default(),
                callee: Callee::Expr(Box::new(Expr::Ident(cov_fn_ident.clone()))),
                args: vec![],
                type_args: None,
            })),
        }),
    ]
}
