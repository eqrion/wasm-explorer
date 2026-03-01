use crate::Span;

/// An export entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    /// Source span.
    pub span: Span,
    /// The exported name.
    pub name: String,
    /// The kind of item being exported.
    pub kind: ExternalKind,
    /// Index of the exported item within its index space.
    pub index: u32,
}

/// The kind of an exported item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalKind {
    /// A function export.
    Func,
    /// A table export.
    Table,
    /// A memory export.
    Memory,
    /// A global export.
    Global,
    /// A tag export.
    Tag,
}

impl From<wasmparser::ExternalKind> for ExternalKind {
    fn from(k: wasmparser::ExternalKind) -> Self {
        match k {
            wasmparser::ExternalKind::Func | wasmparser::ExternalKind::FuncExact => {
                ExternalKind::Func
            }
            wasmparser::ExternalKind::Table => ExternalKind::Table,
            wasmparser::ExternalKind::Memory => ExternalKind::Memory,
            wasmparser::ExternalKind::Global => ExternalKind::Global,
            wasmparser::ExternalKind::Tag => ExternalKind::Tag,
        }
    }
}
