use swc_core::common::{SourceMapper, Span};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::ecma::ast::Program;
use swc_core::plugin::metadata::TransformPluginMetadataContextKind;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use swc_coverage_instrument::{create_coverage_instrumentation_visitor, Range};

#[plugin_transform]
pub fn process_transform(mut program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let filename = metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap_or_else(|| "-".to_string());
    let _cwd = metadata
        .get_context(&TransformPluginMetadataContextKind::Cwd)
        .unwrap_or_else(|| ".".to_string());

    let source_map = metadata.source_map.clone();
    let get_range = move |span: &Span| -> Range {
        if span.hi.is_dummy() || span.lo.is_dummy() {
            return Range::default();
        }
        let lo = source_map.lookup_char_pos(span.lo);
        let hi = source_map.lookup_char_pos(span.hi);
        Range::new(
            lo.line as u32,
            lo.col.0 as u32,
            hi.line as u32,
            hi.col.0 as u32,
        )
    };

    let mut visitor = create_coverage_instrumentation_visitor(&filename, get_range);
    program.visit_mut_with(&mut visitor);
    program
}
