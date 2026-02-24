mod source_coverage;
mod coverage_template;
mod visitors;

pub use source_coverage::{Range, SourceCoverage};
pub use visitors::coverage_visitor::create_coverage_instrumentation_visitor;
