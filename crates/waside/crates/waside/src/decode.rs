use crate::ast::const_expr::ConstExpr;
use crate::ast::custom::CustomSection;
use crate::ast::data::{Data, DataKind};
use crate::ast::elements::{Element, ElementItems, ElementKind};
use crate::ast::exports::{Export, ExternalKind};
use crate::ast::functions::{Func, FuncBody, FuncBodyDef};
use crate::ast::globals::Global;
use crate::ast::imports::{Import, ImportType, TagKind, TagType};
use crate::ast::tags::Tag;
use crate::ast::instructions::Instruction;
use crate::ast::memories::Memory;
use crate::ast::module::Module;
use crate::ast::tables::Table;
use crate::ast::types::{
    ArrayType, CompositeInnerType, CompositeType, ContType, FieldType, FuncType, RecGroup,
    StorageType, StructType, SubType,
};
use crate::error::{Error, Result};
use crate::Span;

/// Options for decoding a wasm module.
#[derive(Debug, Clone, Default)]
pub struct DecodeOptions {
    /// If true, function bodies are stored as lazy references instead of being parsed.
    pub skeleton: bool,
}

impl Module {
    /// Decode a wasm binary into a Module.
    pub fn decode(bytes: &[u8]) -> Result<Module> {
        Self::decode_with_options(bytes, &DecodeOptions::default())
    }

    /// Decode a wasm binary with the given options.
    pub fn decode_with_options(bytes: &[u8], options: &DecodeOptions) -> Result<Module> {
        // Validate the module first
        wasmparser::Validator::new().validate_all(bytes)?;

        let mut module = Module::new();

        let parser = wasmparser::Parser::new(0);
        let mut _code_section_range: Option<std::ops::Range<usize>> = None;
        let mut custom_section_place: Option<&'static str> = Some("before first");

        for payload in parser.parse_all(bytes) {
            let payload = payload?;
            match payload {
                wasmparser::Payload::Version { .. } => {}
                wasmparser::Payload::TypeSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_type_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after type");
                    }
                }
                wasmparser::Payload::ImportSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_import_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after import");
                    }
                }
                wasmparser::Payload::FunctionSection(reader) => {
                    decode_function_section(&mut module, reader)?;
                    // Function section never produces printed output
                }
                wasmparser::Payload::TableSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_table_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after table");
                    }
                }
                wasmparser::Payload::MemorySection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_memory_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after memory");
                    }
                }
                wasmparser::Payload::TagSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_tag_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after tag");
                    }
                }
                wasmparser::Payload::GlobalSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_global_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after global");
                    }
                }
                wasmparser::Payload::ExportSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_export_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after export");
                    }
                }
                wasmparser::Payload::StartSection { func, range: _ } => {
                    module.start = Some(func);
                    custom_section_place = Some("after start");
                }
                wasmparser::Payload::ElementSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_element_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after elem");
                    }
                }
                wasmparser::Payload::DataCountSection { count, range: _ } => {
                    module.data_count = Some(count);
                    // DataCount doesn't produce visible output
                }
                wasmparser::Payload::DataSection(reader) => {
                    let has_content = reader.count() > 0;
                    decode_data_section(&mut module, reader)?;
                    if has_content {
                        custom_section_place = Some("after data");
                    }
                }
                wasmparser::Payload::CodeSectionStart { count, range, .. } => {
                    _code_section_range = Some(range);
                    module.bodies.reserve(count as usize);
                    if count > 0 {
                        custom_section_place = Some("after code");
                    }
                }
                wasmparser::Payload::CodeSectionEntry(body) => {
                    decode_code_entry(&mut module, body, options)?;
                }
                wasmparser::Payload::CustomSection(reader) => {
                    decode_custom_section(&mut module, reader, custom_section_place)?;
                }
                wasmparser::Payload::End(_) => {}
                // Skip component model sections
                _ => {}
            }
        }

        Ok(module)
    }

    /// Decode a single function body, storing the result back into the module. For lazy
    /// bodies, `bytes` must be the original module binary that was passed to `decode`.
    /// Returns a reference to the stored definition.
    pub fn decode_function(&mut self, idx: usize, bytes: &[u8]) -> Result<&FuncBodyDef> {
        if let FuncBody::Lazy { offset, len } = self.bodies[idx] {
            let body_bytes = &bytes[offset..offset + len];
            let reader = wasmparser::FunctionBody::new(wasmparser::BinaryReader::new(
                body_bytes, offset,
            ));
            let def = decode_function_body(reader)?;
            self.bodies[idx] = FuncBody::Decoded(def);
        }
        match &self.bodies[idx] {
            FuncBody::Decoded(def) => Ok(def),
            FuncBody::Lazy { .. } => unreachable!(),
        }
    }
}

fn decode_type_section(module: &mut Module, reader: wasmparser::TypeSectionReader) -> Result<()> {
    for rec_group in reader {
        let rec_group = rec_group?;
        let is_explicit = rec_group.is_explicit_rec_group();
        let mut types = Vec::new();
        let mut first_offset = 0;
        let mut last_end = 0;
        for (offset, sub_type) in rec_group.into_types_and_offsets() {
            if types.is_empty() {
                first_offset = offset;
            }
            last_end = offset;
            types.push(convert_sub_type(sub_type));
        }
        let span = Span::new(first_offset, last_end.saturating_sub(first_offset));
        module.types.push(RecGroup {
            span,
            types,
            is_explicit,
        });
    }
    Ok(())
}

fn convert_sub_type(sub: wasmparser::SubType) -> SubType {
    SubType {
        is_final: sub.is_final,
        supertype_idx: sub.supertype_idx.map(|idx| idx.as_module_index().unwrap()),
        composite_type: convert_composite_type(sub.composite_type),
    }
}

fn convert_composite_type(ct: wasmparser::CompositeType) -> CompositeType {
    CompositeType {
        shared: ct.shared,
        inner: match ct.inner {
            wasmparser::CompositeInnerType::Func(f) => CompositeInnerType::Func(FuncType {
                params: f.params().to_vec(),
                results: f.results().to_vec(),
            }),
            wasmparser::CompositeInnerType::Array(a) => CompositeInnerType::Array(ArrayType {
                field_type: convert_field_type(a.0),
            }),
            wasmparser::CompositeInnerType::Struct(s) => CompositeInnerType::Struct(StructType {
                fields: s.fields.iter().map(|f| convert_field_type(*f)).collect(),
            }),
            wasmparser::CompositeInnerType::Cont(c) => CompositeInnerType::Cont(ContType {
                type_index: c.0.as_module_index().unwrap(),
            }),
        },
    }
}

fn convert_field_type(ft: wasmparser::FieldType) -> FieldType {
    FieldType {
        mutable: ft.mutable,
        element_type: match ft.element_type {
            wasmparser::StorageType::I8 => StorageType::I8,
            wasmparser::StorageType::I16 => StorageType::I16,
            wasmparser::StorageType::Val(v) => StorageType::Val(v),
        },
    }
}

fn decode_import_section(
    module: &mut Module,
    reader: wasmparser::ImportSectionReader,
) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_imports_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, import) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        let ty = match import.ty {
            wasmparser::TypeRef::Func(idx) | wasmparser::TypeRef::FuncExact(idx) => {
                ImportType::Func(idx)
            }
            wasmparser::TypeRef::Table(t) => ImportType::Table(t),
            wasmparser::TypeRef::Memory(m) => ImportType::Memory(m),
            wasmparser::TypeRef::Global(g) => ImportType::Global(g),
            wasmparser::TypeRef::Tag(t) => ImportType::Tag(TagType {
                kind: TagKind::Exception,
                func_type_idx: t.func_type_idx,
            }),
        };
        module.imports.push(Import {
            span: Span::new(start, end - start),
            module: import.module.to_string(),
            name: import.name.to_string(),
            ty,
        });
    }
    Ok(())
}

fn decode_function_section(
    module: &mut Module,
    reader: wasmparser::FunctionSectionReader,
) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, type_index) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        module.functions.push(Func {
            span: Span::new(start, end - start),
            type_index,
        });
    }
    Ok(())
}

fn decode_table_section(module: &mut Module, reader: wasmparser::TableSectionReader) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, table) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        let init = match &table.init {
            wasmparser::TableInit::Expr(expr) => Some(decode_const_expr(expr.clone())?),
            wasmparser::TableInit::RefNull => None,
        };
        module.tables.push(Table {
            span: Span::new(start, end - start),
            ty: table.ty,
            init,
        });
    }
    Ok(())
}

fn decode_memory_section(
    module: &mut Module,
    reader: wasmparser::MemorySectionReader,
) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, memory) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        module.memories.push(Memory {
            span: Span::new(start, end - start),
            ty: memory,
        });
    }
    Ok(())
}

fn decode_tag_section(module: &mut Module, reader: wasmparser::TagSectionReader) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, tag) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        module.tags.push(Tag {
            span: Span::new(start, end - start),
            ty: TagType {
                kind: TagKind::Exception,
                func_type_idx: tag.func_type_idx,
            },
        });
    }
    Ok(())
}

fn decode_global_section(
    module: &mut Module,
    reader: wasmparser::GlobalSectionReader,
) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, global) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        let init_expr = decode_const_expr(global.init_expr)?;
        module.globals.push(Global {
            span: Span::new(start, end - start),
            ty: global.ty,
            init_expr,
        });
    }
    Ok(())
}

fn decode_export_section(
    module: &mut Module,
    reader: wasmparser::ExportSectionReader,
) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, export) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        module.exports.push(Export {
            span: Span::new(start, end - start),
            name: export.name.to_string(),
            kind: ExternalKind::from(export.kind),
            index: export.index,
        });
    }
    Ok(())
}

fn decode_element_section(
    module: &mut Module,
    reader: wasmparser::ElementSectionReader,
) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, element) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        let kind = match element.kind {
            wasmparser::ElementKind::Passive => ElementKind::Passive,
            wasmparser::ElementKind::Active {
                table_index,
                offset_expr,
            } => ElementKind::Active {
                table_index,
                offset_expr: decode_const_expr(offset_expr)?,
            },
            wasmparser::ElementKind::Declared => ElementKind::Declared,
        };
        let items = match element.items {
            wasmparser::ElementItems::Functions(reader) => {
                let funcs: std::result::Result<Vec<u32>, _> = reader.into_iter().collect();
                ElementItems::Functions(funcs?)
            }
            wasmparser::ElementItems::Expressions(ref_type, reader) => {
                let exprs: std::result::Result<Vec<_>, _> = reader
                    .into_iter()
                    .map(|e| {
                        e.and_then(|e| {
                            decode_const_expr(e).map_err(|e| match e {
                                Error::BinaryReader(e) => e,
                                _ => unreachable!(),
                            })
                        })
                    })
                    .collect();
                ElementItems::Expressions(ref_type, exprs?)
            }
        };
        module.elements.push(Element {
            span: Span::new(start, end - start),
            kind,
            items,
        });
    }
    Ok(())
}

fn decode_data_section(module: &mut Module, reader: wasmparser::DataSectionReader) -> Result<()> {
    let section_end = reader.range().end;
    let mut iter = reader.into_iter_with_offsets().peekable();
    while let Some(result) = iter.next() {
        let (start, data) = result?;
        let end = iter
            .peek()
            .and_then(|r| r.as_ref().ok().map(|(off, _)| *off))
            .unwrap_or(section_end);
        let kind = match data.kind {
            wasmparser::DataKind::Passive => DataKind::Passive,
            wasmparser::DataKind::Active {
                memory_index,
                offset_expr,
            } => DataKind::Active {
                memory_index,
                offset_expr: decode_const_expr(offset_expr)?,
            },
        };
        module.data.push(Data {
            span: Span::new(start, end - start),
            kind,
            data: data.data.to_vec(),
        });
    }
    Ok(())
}

fn decode_code_entry(
    module: &mut Module,
    body: wasmparser::FunctionBody,
    options: &DecodeOptions,
) -> Result<()> {
    if options.skeleton {
        let range = body.range();
        module.bodies.push(FuncBody::Lazy {
            offset: range.start,
            len: range.end - range.start,
        });
    } else {
        let def = decode_function_body(body)?;
        module.bodies.push(FuncBody::Decoded(def));
    }
    Ok(())
}

fn decode_function_body(body: wasmparser::FunctionBody) -> Result<FuncBodyDef> {
    let range = body.range();
    let span = Span::new(range.start, range.end - range.start);

    let mut locals = Vec::new();
    let local_reader = body.get_locals_reader()?;
    for local in local_reader {
        let (count, val_type) = local?;
        locals.push((count, val_type));
    }

    let mut instructions = Vec::new();
    let mut ops_reader = body.get_operators_reader()?;
    while !ops_reader.eof() {
        let pos = ops_reader.original_position();
        let op = ops_reader.read()?;
        let end = ops_reader.original_position();
        let instr = Instruction::from_operator(&op);
        instructions.push((Span::new(pos, end - pos), instr));
    }

    Ok(FuncBodyDef {
        span,
        locals,
        instructions,
    })
}

fn decode_const_expr(expr: wasmparser::ConstExpr) -> Result<ConstExpr> {
    let range = expr.get_binary_reader().range();
    let span = Span::new(range.start, range.end - range.start);
    let mut ops = Vec::new();
    let mut reader = expr.get_operators_reader();
    while !reader.eof() {
        let op = reader.read()?;
        match op {
            wasmparser::Operator::End => break,
            _ => ops.push(Instruction::from_operator(&op)),
        }
    }
    Ok(ConstExpr { span, ops })
}

fn decode_custom_section(
    module: &mut Module,
    reader: wasmparser::CustomSectionReader,
    placement: Option<&str>,
) -> Result<()> {
    match reader.as_known() {
        wasmparser::KnownCustom::Name(name_reader) => {
            decode_name_section(module, name_reader)?;
        }
        _ => {
            let range = reader.range();
            module.custom_sections.push(CustomSection {
                span: Span::new(range.start, range.end - range.start),
                name: reader.name().to_string(),
                data: reader.data().to_vec(),
                placement: placement.map(|s| s.to_string()),
            });
        }
    }
    Ok(())
}

fn decode_name_section(module: &mut Module, reader: wasmparser::NameSectionReader) -> Result<()> {
    for name in reader {
        match name? {
            wasmparser::Name::Module { name, .. } => {
                module.names.module_name = Some(name.to_string());
            }
            wasmparser::Name::Function(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .function_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Local(indirect) => {
                for func_names in indirect {
                    let func_names = func_names?;
                    let func_idx = func_names.index;
                    let mut local_map = std::collections::HashMap::new();
                    for naming in func_names.names {
                        let naming = naming?;
                        local_map.insert(naming.index, naming.name.to_string());
                    }
                    module.names.local_names.insert(func_idx, local_map);
                }
            }
            wasmparser::Name::Label(indirect) => {
                for func_labels in indirect {
                    let func_labels = func_labels?;
                    let func_idx = func_labels.index;
                    let mut label_map = std::collections::HashMap::new();
                    for naming in func_labels.names {
                        let naming = naming?;
                        label_map.insert(naming.index, naming.name.to_string());
                    }
                    module.names.label_names.insert(func_idx, label_map);
                }
            }
            wasmparser::Name::Type(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .type_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Table(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .table_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Memory(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .memory_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Global(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .global_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Element(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .element_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Data(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .data_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Tag(map) => {
                for naming in map {
                    let naming = naming?;
                    module
                        .names
                        .tag_names
                        .insert(naming.index, naming.name.to_string());
                }
            }
            wasmparser::Name::Field(indirect) => {
                for type_fields in indirect {
                    let type_fields = type_fields?;
                    let type_idx = type_fields.index;
                    let mut field_map = std::collections::HashMap::new();
                    for naming in type_fields.names {
                        let naming = naming?;
                        field_map.insert(naming.index, naming.name.to_string());
                    }
                    module.names.field_names.insert(type_idx, field_map);
                }
            }
            wasmparser::Name::Unknown { .. } => {}
        }
    }
    Ok(())
}
