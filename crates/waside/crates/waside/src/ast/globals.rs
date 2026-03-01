use crate::ast::const_expr::ConstExpr;
use crate::Span;

/// A global entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    /// Source span.
    pub span: Span,
    /// The global's value type and mutability.
    pub ty: wasmparser::GlobalType,
    /// Constant expression used to initialize the global.
    pub init_expr: ConstExpr,
}
