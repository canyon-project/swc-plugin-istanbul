use swc_core::ecma::ast::Program;
use swc_core::plugin::metadata::TransformPluginMetadataContextKind;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use swc_coverage_instrument::create_coverage_instrumentation_visitor;

#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let filename = metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap_or_else(|| "-".to_string());
    let _cwd = metadata
        .get_context(&TransformPluginMetadataContextKind::Cwd)
        .unwrap_or_else(|| ".".to_string());

    create_coverage_instrumentation_visitor(&filename);
    program
}
