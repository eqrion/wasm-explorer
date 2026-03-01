use crate::ast::instructions::Instruction;
use crate::Span;

/// A function declaration (from the function section) — just a type index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Func {
    /// Source span.
    pub span: Span,
    /// Index into the type section for this function's signature.
    pub type_index: u32,
}

/// A function body, either fully decoded or lazy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuncBody {
    /// Fully decoded function body.
    Decoded(FuncBodyDef),
    /// Not yet decoded; the fields locate the body bytes in the original binary.
    Lazy {
        /// Byte offset of the function body in the original binary.
        offset: usize,
        /// Length of the function body in bytes.
        len: usize,
    },
}

/// A fully decoded function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncBodyDef {
    /// Source span of the entire function body.
    pub span: Span,
    /// Local variable declarations as `(count, type)` pairs.
    pub locals: Vec<(u32, wasmparser::ValType)>,
    /// Decoded instructions paired with their source spans.
    pub instructions: Vec<(Span, Instruction)>,
}
