use crate::Span;

/// A recursive type group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecGroup {
    /// Source span.
    pub span: Span,
    /// The subtypes in this group.
    pub types: Vec<SubType>,
    /// Whether the `rec` keyword was written explicitly in the source.
    pub is_explicit: bool,
}

/// A subtype definition within a rec group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubType {
    /// Whether the subtype is marked `final` (not extensible).
    pub is_final: bool,
    /// Index of the declared supertype, if any.
    pub supertype_idx: Option<u32>,
    /// The underlying composite type.
    pub composite_type: CompositeType,
}

/// A composite type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeType {
    /// The concrete type definition.
    pub inner: CompositeInnerType,
    /// Whether the type is marked `shared` (for shared-everything threads).
    pub shared: bool,
}

/// The inner type of a composite type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositeInnerType {
    /// A function type.
    Func(FuncType),
    /// An array type (GC proposal).
    Array(ArrayType),
    /// A struct type (GC proposal).
    Struct(StructType),
    /// A continuation type (stack-switching proposal).
    Cont(ContType),
}

/// A function type signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncType {
    /// Parameter types.
    pub params: Vec<wasmparser::ValType>,
    /// Result types.
    pub results: Vec<wasmparser::ValType>,
}

/// An array type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayType {
    /// The element field type.
    pub field_type: FieldType,
}

/// A struct type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    /// The ordered list of field types.
    pub fields: Vec<FieldType>,
}

/// A field type used in structs and arrays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldType {
    /// The storage type of this field.
    pub element_type: StorageType,
    /// Whether the field is mutable.
    pub mutable: bool,
}

/// Storage type for fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    /// Packed 8-bit integer.
    I8,
    /// Packed 16-bit integer.
    I16,
    /// A full value type.
    Val(wasmparser::ValType),
}

/// A continuation type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContType {
    /// Index of the function type this continuation wraps.
    pub type_index: u32,
}
