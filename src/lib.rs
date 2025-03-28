use swc_core::ecma::ast::{Ident, PropOrSpread};
use swc_core::ecma::{
    ast::{
        Expr, KeyValueProp, ObjectLit, Program,
        Prop,
        PropName
    },
    transforms::testing::test_inline,
    visit::{as_folder, FoldWith, VisitMut},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub dsn: Option<String>,
    pub reporter: Option<String>,
    pub instrumentCwd: Option<String>,
    pub branch: Option<String>,
    pub sha: Option<String>,
    pub projectID: Option<String>,
    pub compareTarget: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dsn: None,
            reporter: None,
            instrumentCwd: None,
            branch: None,
            sha: None,
            projectID: None,
            compareTarget: None,
        }
    }
}

pub struct TransformVisitor {
    config: Config,
}

impl TransformVisitor {
    pub fn new() -> Self {
        Self { config: Config::default() }
    }
    // 处理对象字面量属性的函数
    fn process_coverage_data_object(&mut self, obj: &mut ObjectLit) {
        let excluded_keys = ["statementMap", "fnMap", "branchMap", "inputSourceMap"];

        let dsn = self.config.dsn.clone().unwrap_or("".to_string());
        let reporter = self.config.reporter.clone().unwrap_or("".to_string());
        let instrumentCwd = self.config.instrumentCwd.clone().unwrap_or("".to_string());
        let branch = self.config.branch.clone().unwrap_or("".to_string());
        let sha = self.config.sha.clone().unwrap_or("".to_string());
        let projectID = self.config.projectID.clone().unwrap_or("".to_string());
        let compareTarget = self.config.compareTarget.clone().unwrap_or("".to_string());

        // 过滤掉指定的属性
        obj.props.retain(|prop| {
            match prop {
                PropOrSpread::Prop(prop) => {
                    if let Prop::KeyValue(KeyValueProp {
                                              key: PropName::Ident(Ident { sym, .. }),
                                              ..
                                          }) = &**prop {
                        // 排除指定的属性名
                        // !excluded_keys.contains(&sym.as_ref())
                        // TODO 暂时不排除任何属性
                        true
                    } else {
                        true
                    }
                }
                _ => true,
            }
        });

        // Add new properties from config
        let mut new_props = vec![];
        if!dsn.is_empty() {
            new_props.push(self.create_string_prop("dsn", dsn));
        }
        if!reporter.is_empty() {
            new_props.push(self.create_string_prop("reporter", reporter));
        }
        if!instrumentCwd.is_empty() {
            new_props.push(self.create_string_prop("instrumentCwd", instrumentCwd));
        }
        if!branch.is_empty() {
            new_props.push(self.create_string_prop("branch", branch));
        }
        if!sha.is_empty() {
            new_props.push(self.create_string_prop("sha", sha));
        }
        if!projectID.is_empty() {
            new_props.push(self.create_string_prop("projectID", projectID));
        }
        if!compareTarget.is_empty() {
            new_props.push(self.create_string_prop("compareTarget", compareTarget));
        }

        // Extend the object with new properties
        obj.props.extend(new_props);
    }

    // Helper function to create a KeyValueProp with string value
    fn create_string_prop(&self, key: &str, value: String) -> PropOrSpread {
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident {
                sym: key.into(),
                span: Default::default(),
                optional: false,
            }),
            value: Box::new(Expr::Lit(swc_core::ecma::ast::Lit::Str(swc_core::ecma::ast::Str {
                value: value.into(),
                span: Default::default(),
                raw: None,
            }))),
        })))
    }
}

impl VisitMut for TransformVisitor {
    // 遍历每个表达式时，修改表达式
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        match expr {
            // 当表达式是对象字面量时
            Expr::Object(ref mut obj) => {
                // 调用 visit_mut_object_lit 来处理对象字面量的属性
                self.visit_mut_object_lit(obj);
            }
            _ => {}
        }
    }

    fn visit_mut_object_lit(&mut self, obj: &mut ObjectLit) {
        // 定义需要同时包含的属性
        let required_keys = ["statementMap", "fnMap", "branchMap"];

        // 检查对象字面量是否同时包含这些属性
        let contains_required_keys = required_keys.iter().all(|&key| {
            obj.props.iter().any(|prop| {
                if let PropOrSpread::Prop(ref prop) = prop {
                    if let Prop::KeyValue(KeyValueProp {
                                              key: PropName::Ident(Ident { sym, .. }),
                                              ..
                                          }) = &**prop {
                        return sym.as_ref() == key;
                    }
                }
                false
            })
        });

        // 只有在同时包含所有指定属性时才进行处理
        if contains_required_keys {
            self.process_coverage_data_object(obj);
        }
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config = serde_json::from_str::<Option<Config>>(
        &metadata
            .get_transform_plugin_config()
            .expect("failed to get plugin config for react-remove-properties"),
    )
        .expect("invalid config for react-remove-properties")
        .unwrap_or_default(); // Use default if config is None
    program.fold_with(&mut as_folder(TransformVisitor { config }))
}

test_inline!(
    Default::default(),
    |_| as_folder(TransformVisitor {config: Config::default()}),
    boo,
    // 输入代码
    r#"const coverageData={fnMap:"nihao",statementMap:"",branchMap:"sss"};"#,
    // 经插件转换后的输出代码
    r#"const coverageData={};"#
);