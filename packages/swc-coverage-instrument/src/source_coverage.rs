//! Istanbul 格式的 coverage 数据结构，学 old 的 source_coverage

use indexmap::IndexMap;

/// 位置 { line, column }
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

/// 范围 { start, end }
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Range {
    pub start: Location,
    pub end: Location,
}

impl Range {
    pub fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Self {
        Self {
            start: Location {
                line: start_line,
                column: start_column,
            },
            end: Location {
                line: end_line,
                column: end_column,
            },
        }
    }
}

/// 收集 statement 的 coverage 数据，学 old 的 SourceCoverage
#[derive(Clone, Debug, Default)]
pub struct SourceCoverage {
    pub statement_map: IndexMap<u32, Range>,
    pub s: IndexMap<u32, u32>,
    next_id: u32,
}

impl SourceCoverage {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加 statement，返回 id
    pub fn new_statement(&mut self, loc: &Range) -> u32 {
        let id = self.next_id;
        self.statement_map.insert(id, *loc);
        self.s.insert(id, 0);
        self.next_id += 1;
        id
    }
}
