use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use swc_core::{
    common::{Span, Spanned, util::take::Take, DUMMY_SP},
    ecma::{
        ast::*,
        utils::IsDirective,
        visit::{VisitMut, VisitMutWith},
    },
};

use crate::source_coverage::{Range, SourceCoverage};
use crate::coverage_template;

/// 创建 cov_xxx().s[id]++ 表达式
fn create_increase_counter_expr(id: u32, cov_fn_ident: &Ident) -> Expr {
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

/// 覆盖率插桩 Visitor
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

    /// 创建覆盖率初始化语句
    fn create_coverage_init_stmts(&self) -> Vec<Stmt> {
        let cov = self.cov.borrow();
        coverage_template::create_coverage_init_stmts(&self.filename, &self.cov_fn_ident, &cov)
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

        let stmts = self.create_coverage_init_stmts();
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
