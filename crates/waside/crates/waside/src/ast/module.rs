/// Definition identifier for linking between AST items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemId {
    /// A type index.
    Type(u32),
    /// A function index.
    Func(u32),
    /// A table index.
    Table(u32),
    /// A memory index.
    Memory(u32),
    /// A global index.
    Global(u32),
    /// An element segment index.
    Element(u32),
    /// A data segment index.
    Data(u32),
    /// A local variable index within a function.
    Local {
        /// Index of the enclosing function.
        func: u32,
        /// Index of the local variable within the function.
        local: u32,
    },
    /// A tag index.
    Tag(u32),
}

/// Trait for AST items that can be identified by an [`ItemId`].
pub trait Item {
    /// Print this item as a WAT section entry.
    fn print(
        &self,
        ctx: &crate::print::PrintContext,
        p: &mut dyn crate::printer::Printer,
        idx: u32,
    );
}

use crate::ast::custom::CustomSection;
use crate::ast::data::Data;
use crate::ast::elements::Element;
use crate::ast::exports::Export;
use crate::ast::functions::{Func, FuncBody};
use crate::ast::globals::Global;
use crate::ast::imports::{Import, ImportType};
use crate::ast::tags::Tag;
use crate::Span;
use crate::ast::memories::Memory;
use crate::ast::names::NameSection;
use crate::ast::tables::Table;
use crate::ast::types::RecGroup;

/// A decoded WebAssembly module.
#[derive(Debug, Clone, Eq)]
pub struct Module {
    /// Type section: recursive type groups.
    pub types: Vec<RecGroup>,
    /// Import section.
    pub imports: Vec<Import>,
    /// Function section: type indices for each function.
    pub functions: Vec<Func>,
    /// Table section.
    pub tables: Vec<Table>,
    /// Memory section.
    pub memories: Vec<Memory>,
    /// Tag section.
    pub tags: Vec<Tag>,
    /// Global section.
    pub globals: Vec<Global>,
    /// Export section.
    pub exports: Vec<Export>,
    /// Start function index.
    pub start: Option<u32>,
    /// Element section.
    pub elements: Vec<Element>,
    /// Data count (from data count section).
    pub data_count: Option<u32>,
    /// Data section.
    pub data: Vec<Data>,
    /// Function bodies (code section).
    pub bodies: Vec<FuncBody>,
    /// Parsed name section.
    pub names: NameSection,
    /// Other custom sections.
    pub custom_sections: Vec<CustomSection>,
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.types == other.types
            && self.imports == other.imports
            && self.functions == other.functions
            && self.tables == other.tables
            && self.memories == other.memories
            && self.tags == other.tags
            && self.globals == other.globals
            && self.exports == other.exports
            && self.start == other.start
            && self.elements == other.elements
            && self.data_count == other.data_count
            && self.data == other.data
            && self.bodies == other.bodies
            && self.names == other.names
            && self.custom_sections == other.custom_sections
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

impl Module {
    /// Create a new empty module.
    pub fn new() -> Self {
        Module {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            tags: Vec::new(),
            globals: Vec::new(),
            exports: Vec::new(),
            start: None,
            elements: Vec::new(),
            data_count: None,
            data: Vec::new(),
            bodies: Vec::new(),
            names: NameSection::default(),
            custom_sections: Vec::new(),
        }
    }
}


impl Module {
    /// Find the closest enclosing AST item for `id`.
    ///
    /// For most variants this is the item directly identified by the index.
    /// For [`ItemId::Local`] it returns the enclosing function.
    pub fn find_closest_item(&self, id: &ItemId) -> Option<&dyn Item> {
        match id {
            ItemId::Type(i) => {
                let mut remaining = *i as usize;
                for group in &self.types {
                    if remaining < group.types.len() {
                        return Some(&group.types[remaining]);
                    }
                    remaining -= group.types.len();
                }
                None
            }
            ItemId::Func(i) => self.functions.get(*i as usize).map(|x| x as &dyn Item),
            ItemId::Table(i) => self.tables.get(*i as usize).map(|x| x as &dyn Item),
            ItemId::Memory(i) => self.memories.get(*i as usize).map(|x| x as &dyn Item),
            ItemId::Global(i) => self.globals.get(*i as usize).map(|x| x as &dyn Item),
            ItemId::Element(i) => self.elements.get(*i as usize).map(|x| x as &dyn Item),
            ItemId::Data(i) => self.data.get(*i as usize).map(|x| x as &dyn Item),
            ItemId::Local { func, .. } => {
                self.functions.get(*func as usize).map(|x| x as &dyn Item)
            }
            ItemId::Tag(i) => self.tags.get(*i as usize).map(|x| x as &dyn Item),
        }
    }

    /// Return an `(ItemId, Span)` pair for every definition in the module, in
    /// section order. Locals are excluded.
    pub fn definitions(&self) -> Vec<(ItemId, Span)> {
        let mut result = Vec::new();

        // Types — each subtype borrows its parent RecGroup's span.
        let mut type_idx = 0u32;
        for group in &self.types {
            for _ in &group.types {
                result.push((ItemId::Type(type_idx), group.span));
                type_idx += 1;
            }
        }

        // Imports — track per-kind indices to produce the correct absolute index.
        let mut func_idx = 0u32;
        let mut table_idx = 0u32;
        let mut memory_idx = 0u32;
        let mut global_idx = 0u32;
        let mut tag_idx = 0u32;
        for import in &self.imports {
            let id = match &import.ty {
                ImportType::Func(_) => {
                    let id = ItemId::Func(func_idx);
                    func_idx += 1;
                    id
                }
                ImportType::Table(_) => {
                    let id = ItemId::Table(table_idx);
                    table_idx += 1;
                    id
                }
                ImportType::Memory(_) => {
                    let id = ItemId::Memory(memory_idx);
                    memory_idx += 1;
                    id
                }
                ImportType::Global(_) => {
                    let id = ItemId::Global(global_idx);
                    global_idx += 1;
                    id
                }
                ImportType::Tag(_) => {
                    let id = ItemId::Tag(tag_idx);
                    tag_idx += 1;
                    id
                }
            };
            result.push((id, import.span));
        }

        // Non-imported definitions.
        for (i, func) in self.functions.iter().enumerate() {
            result.push((ItemId::Func(func_idx + i as u32), func.span));
        }
        for (i, table) in self.tables.iter().enumerate() {
            result.push((ItemId::Table(table_idx + i as u32), table.span));
        }
        for (i, memory) in self.memories.iter().enumerate() {
            result.push((ItemId::Memory(memory_idx + i as u32), memory.span));
        }
        for (i, global) in self.globals.iter().enumerate() {
            result.push((ItemId::Global(global_idx + i as u32), global.span));
        }
        for (i, tag) in self.tags.iter().enumerate() {
            result.push((ItemId::Tag(tag_idx + i as u32), tag.span));
        }
        for (i, elem) in self.elements.iter().enumerate() {
            result.push((ItemId::Element(i as u32), elem.span));
        }
        for (i, data) in self.data.iter().enumerate() {
            result.push((ItemId::Data(i as u32), data.span));
        }

        result
    }
}
