use crate::ast::const_expr::ConstExpr;
use crate::Span;

/// An element segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Source span.
    pub span: Span,
    /// The kind of element segment (passive, active, or declared).
    pub kind: ElementKind,
    /// The element items (function indices or constant expressions).
    pub items: ElementItems,
}

/// The kind of element segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementKind {
    /// Not associated with any table; can be used with `table.init`.
    Passive,
    /// Copied into a table at module instantiation.
    Active {
        /// The target table index, or `None` for the implicit table 0.
        table_index: Option<u32>,
        /// Byte offset within the table.
        offset_expr: ConstExpr,
    },
    /// Declared but not accessible at runtime; used to declare ref.func operands.
    Declared,
}

/// The items in an element segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementItems {
    /// Function indices.
    Functions(Vec<u32>),
    /// Constant expressions with a ref type.
    Expressions(wasmparser::RefType, Vec<ConstExpr>),
}
