use crate::ast::const_expr::ConstExpr;
use crate::Span;

/// A data segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    /// Source span.
    pub span: Span,
    /// The kind of data segment (passive or active).
    pub kind: DataKind,
    /// The raw bytes of the segment.
    pub data: Vec<u8>,
}

/// The kind of data segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataKind {
    /// Not associated with a memory; can be used with `memory.init`.
    Passive,
    /// Copied into a memory at module instantiation.
    Active {
        /// Index of the target memory.
        memory_index: u32,
        /// Byte offset within the memory.
        offset_expr: ConstExpr,
    },
}
