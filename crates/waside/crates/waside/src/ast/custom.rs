use crate::Span;

/// A custom section (other than the name section).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomSection {
    /// Source span.
    pub span: Span,
    /// The custom section name.
    pub name: String,
    /// The raw bytes of the custom section payload.
    pub data: Vec<u8>,
    /// Placement hint for WAT printing (e.g. "before first", "after type").
    pub placement: Option<String>,
}
