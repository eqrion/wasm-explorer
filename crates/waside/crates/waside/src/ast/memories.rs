use crate::Span;

/// A memory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    /// Source span.
    pub span: Span,
    /// The memory type (limits and address width).
    pub ty: wasmparser::MemoryType,
}
