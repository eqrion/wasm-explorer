#![recursion_limit = "2048"]
#![warn(missing_docs)]

//! A WebAssembly binary decoder, encoder, and WAT printer.

mod ast;
mod decode;
mod encode;
mod error;
mod print;
mod printer;
mod span;

pub use ast::const_expr::ConstExpr;
pub use ast::custom::CustomSection;
pub use ast::data::{Data, DataKind};
pub use ast::elements::{Element, ElementItems, ElementKind};
pub use ast::exports::{Export, ExternalKind};
pub use ast::functions::{Func, FuncBody, FuncBodyDef};
pub use ast::globals::Global;
pub use ast::imports::{Import, ImportType, TagKind, TagType};
pub use ast::tags::Tag;
pub use ast::instructions::{BrTableData, Instruction};
pub use ast::memories::Memory;
pub use ast::module::Module;
pub use ast::names::NameSection;
pub use ast::tables::Table;
pub use ast::types::{
    ArrayType, CompositeInnerType, CompositeType, ContType, FieldType, FuncType, RecGroup,
    StorageType, StructType, SubType,
};
pub use decode::DecodeOptions;
pub use error::{Error, Result};
pub use ast::module::{Item, ItemId};
pub use print::PrintContext;
pub use printer::{PlainTextPrinter, Printer, Style};
pub use span::Span;
