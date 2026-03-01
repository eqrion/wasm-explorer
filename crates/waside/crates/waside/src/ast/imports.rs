use crate::Span;

/// An import entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// Source span.
    pub span: Span,
    /// The module string of the two-level import name.
    pub module: String,
    /// The field string of the two-level import name.
    pub name: String,
    /// The type of the imported item.
    pub ty: ImportType,
}

/// The type of an import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportType {
    /// A function import; value is the type-section index.
    Func(u32),
    /// A table import.
    Table(wasmparser::TableType),
    /// A memory import.
    Memory(wasmparser::MemoryType),
    /// A global import.
    Global(wasmparser::GlobalType),
    /// A tag import.
    Tag(TagType),
}

/// A tag type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagType {
    /// The semantic kind of the tag.
    pub kind: TagKind,
    /// Index into the type section for the tag's function type.
    pub func_type_idx: u32,
}

/// The kind of a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// An exception tag (exception-handling proposal).
    Exception,
}
