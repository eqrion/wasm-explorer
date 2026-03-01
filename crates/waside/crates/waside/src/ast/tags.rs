use crate::ast::imports::TagType;
use crate::Span;

/// A tag entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    /// Source span.
    pub span: Span,
    /// The tag type.
    pub ty: TagType,
}
