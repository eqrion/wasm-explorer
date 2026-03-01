use crate::ast::const_expr::ConstExpr;
use crate::Span;

/// A table entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Source span.
    pub span: Span,
    /// The table type (element type and limits).
    pub ty: wasmparser::TableType,
    /// Optional initializer expression (for tables with an inline init).
    pub init: Option<ConstExpr>,
}
