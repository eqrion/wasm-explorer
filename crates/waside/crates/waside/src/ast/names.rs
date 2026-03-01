use std::collections::HashMap;

/// Parsed name section data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NameSection {
    /// Optional module name.
    pub module_name: Option<String>,
    /// Names for function indices.
    pub function_names: HashMap<u32, String>,
    /// Names for local variables, keyed by function index then local index.
    pub local_names: HashMap<u32, HashMap<u32, String>>,
    /// Names for type indices.
    pub type_names: HashMap<u32, String>,
    /// Names for table indices.
    pub table_names: HashMap<u32, String>,
    /// Names for memory indices.
    pub memory_names: HashMap<u32, String>,
    /// Names for global indices.
    pub global_names: HashMap<u32, String>,
    /// Names for element segment indices.
    pub element_names: HashMap<u32, String>,
    /// Names for data segment indices.
    pub data_names: HashMap<u32, String>,
    /// Names for tag indices.
    pub tag_names: HashMap<u32, String>,
    /// Names for struct fields, keyed by type index then field index.
    pub field_names: HashMap<u32, HashMap<u32, String>>,
    /// Names for labels, keyed by function index then label depth.
    pub label_names: HashMap<u32, HashMap<u32, String>>,
}
