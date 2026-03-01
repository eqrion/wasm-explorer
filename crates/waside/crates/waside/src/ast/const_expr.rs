use crate::ast::instructions::Instruction;
use crate::Span;

/// A constant expression (used in globals, element offsets, data offsets, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstExpr {
    /// Source span.
    pub span: Span,
    /// The sequence of constant instructions.
    pub ops: Vec<Instruction>,
}
