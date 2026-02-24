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
use serde_json;

use crate::source_coverage::{Range, SourceCoverage};
use crate::coverage_template;

/// 覆盖率插桩 Visitor
pub struct CoverageVisitor {
    filename: String,
    cov: RefCell<SourceCoverage>,
    cov_fn_ident: Ident,
    get_range: Box<dyn Fn(&Span) -> Range + Send + Sync>,
    ast_json: RefCell<Option<String>>,
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
            ast_json: RefCell::new(None),
        }
    }

    fn get_range(&self, span: &Span) -> Range {
        (self.get_range)(span)
    }

    /// 创建覆盖率初始化语句
    fn create_coverage_init_stmts(&self) -> Vec<Stmt> {
        let cov = self.cov.borrow();
        let ast_json = self.ast_json.borrow();
        coverage_template::create_coverage_init_stmts(
            &self.filename, 
            &self.cov_fn_ident, 
            &cov,
            ast_json.as_deref()
        )
    }
}

impl VisitMut for CoverageVisitor {
    fn visit_mut_program(&mut self, program: &mut Program) {
        println!("=== visit_mut_program: 开始处理程序 ===");
        
        // 序列化 AST 为 JSON
        match serde_json::to_string(program) {
            Ok(json) => {
                println!("  -> AST 序列化成功，长度: {} 字节", json.len());
                *self.ast_json.borrow_mut() = Some(json);
            }
            Err(e) => {
                println!("  -> AST 序列化失败: {}", e);
            }
        }
        
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
}

/// 创建 coverage instrumentation visitor
/// get_range: 将 Span 转为 Istanbul Range，无 source_map 时传入 |_| Range::default()
pub fn create_coverage_instrumentation_visitor<F>(filename: &str, get_range: F) -> CoverageVisitor
where
    F: Fn(&Span) -> Range + Send + Sync + 'static,
{
    CoverageVisitor::new(filename.to_string(), Box::new(get_range))
}
