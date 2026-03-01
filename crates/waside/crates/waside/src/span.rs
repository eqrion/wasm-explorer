/// A byte range within a wasm binary.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    /// Byte offset from the start of the binary.
    pub offset: usize,
    /// Length in bytes.
    pub len: usize,
}

impl Span {
    /// Create a new span at `offset` with the given `len`.
    pub fn new(offset: usize, len: usize) -> Self {
        Span { offset, len }
    }
}

/// Span equality always returns true. This enables `#[derive(PartialEq, Eq)]`
/// on all AST types without spans affecting equality comparisons, since spans
/// change after re-encoding.
impl PartialEq for Span {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Span {}
