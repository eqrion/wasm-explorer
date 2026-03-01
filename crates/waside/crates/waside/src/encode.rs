use crate::ast::const_expr::ConstExpr;
use crate::ast::data::{Data, DataKind};
use crate::ast::elements::{Element, ElementItems, ElementKind};
use crate::ast::exports::ExternalKind;
use crate::ast::functions::{FuncBody, FuncBodyDef};
use crate::ast::imports::{Import, ImportType};
use crate::ast::instructions::Instruction;
use crate::ast::module::Module;
use crate::ast::names::NameSection;
use crate::ast::tables::Table;
use crate::ast::types::{
    CompositeInnerType, CompositeType, FieldType, RecGroup, StorageType, SubType,
};
use crate::error::Result;

impl Module {
    /// Encode this module to a wasm binary. If the module contains lazy function bodies,
    /// `bytes` must be the original module binary that was passed to `decode`.
    pub fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut wasm_module = wasm_encoder::Module::new();

        // Helper to emit custom sections with a given placement
        let emit_custom = |wasm_module: &mut wasm_encoder::Module, placement: &str| {
            for custom in &self.custom_sections {
                if custom.placement.as_deref() == Some(placement) {
                    wasm_module.section(&wasm_encoder::CustomSection {
                        name: (&custom.name).into(),
                        data: (&custom.data).into(),
                    });
                }
            }
        };

        // Custom sections before first
        emit_custom(&mut wasm_module, "before first");

        // Type section
        if !self.types.is_empty() {
            let mut types = wasm_encoder::TypeSection::new();
            for rec_group in &self.types {
                encode_rec_group(&mut types, rec_group);
            }
            wasm_module.section(&types);
        }
        emit_custom(&mut wasm_module, "after type");

        // Import section
        if !self.imports.is_empty() {
            let mut imports = wasm_encoder::ImportSection::new();
            for import in &self.imports {
                encode_import(&mut imports, import);
            }
            wasm_module.section(&imports);
        }
        emit_custom(&mut wasm_module, "after import");

        // Function section
        if !self.functions.is_empty() {
            let mut functions = wasm_encoder::FunctionSection::new();
            for func in &self.functions {
                functions.function(func.type_index);
            }
            wasm_module.section(&functions);
        }
        emit_custom(&mut wasm_module, "after func");

        // Table section
        if !self.tables.is_empty() {
            let mut tables = wasm_encoder::TableSection::new();
            for table in &self.tables {
                encode_table(&mut tables, table);
            }
            wasm_module.section(&tables);
        }
        emit_custom(&mut wasm_module, "after table");

        // Memory section
        if !self.memories.is_empty() {
            let mut memories = wasm_encoder::MemorySection::new();
            for memory in &self.memories {
                memories.memory(encode_memory_type(&memory.ty));
            }
            wasm_module.section(&memories);
        }
        emit_custom(&mut wasm_module, "after memory");

        // Tag section
        if !self.tags.is_empty() {
            let mut tags = wasm_encoder::TagSection::new();
            for tag in &self.tags {
                tags.tag(wasm_encoder::TagType {
                    kind: wasm_encoder::TagKind::Exception,
                    func_type_idx: tag.ty.func_type_idx,
                });
            }
            wasm_module.section(&tags);
        }
        emit_custom(&mut wasm_module, "after tag");

        // Global section
        if !self.globals.is_empty() {
            let mut globals = wasm_encoder::GlobalSection::new();
            for global in &self.globals {
                globals.global(
                    encode_global_type(&global.ty),
                    &encode_const_expr(&global.init_expr),
                );
            }
            wasm_module.section(&globals);
        }
        emit_custom(&mut wasm_module, "after global");

        // Export section
        if !self.exports.is_empty() {
            let mut exports = wasm_encoder::ExportSection::new();
            for export in &self.exports {
                exports.export(&export.name, encode_export_kind(export.kind), export.index);
            }
            wasm_module.section(&exports);
        }
        emit_custom(&mut wasm_module, "after export");

        // Start section
        if let Some(start) = self.start {
            wasm_module.section(&wasm_encoder::StartSection {
                function_index: start,
            });
        }
        emit_custom(&mut wasm_module, "after start");

        // Element section
        if !self.elements.is_empty() {
            let mut elements = wasm_encoder::ElementSection::new();
            for element in &self.elements {
                encode_element(&mut elements, element);
            }
            wasm_module.section(&elements);
        }
        emit_custom(&mut wasm_module, "after elem");

        // Data count section: emit when the module uses data-referencing instructions
        // (memory.init, data.drop, array.new_data, array.init_data) to match wat behavior
        if self.needs_data_count_section() {
            wasm_module.section(&wasm_encoder::DataCountSection {
                count: self.data.len() as u32,
            });
        }
        emit_custom(&mut wasm_module, "after data count");

        // Code section
        if !self.bodies.is_empty() {
            let mut code = wasm_encoder::CodeSection::new();
            for body in &self.bodies {
                match body {
                    FuncBody::Decoded(def) => {
                        let func = encode_function_body(def);
                        code.function(&func);
                    }
                    FuncBody::Lazy { offset, len } => {
                        code.raw(&bytes[*offset..*offset + *len]);
                    }
                }
            }
            wasm_module.section(&code);
        }
        emit_custom(&mut wasm_module, "after code");

        // Data section
        if !self.data.is_empty() {
            let mut data_section = wasm_encoder::DataSection::new();
            for data in &self.data {
                encode_data(&mut data_section, data);
            }
            wasm_module.section(&data_section);
        }
        emit_custom(&mut wasm_module, "after data");

        // Name section
        encode_name_section(&mut wasm_module, &self.names);

        // Custom sections without placement (shouldn't normally happen)
        for custom in &self.custom_sections {
            if custom.placement.is_none() {
                wasm_module.section(&wasm_encoder::CustomSection {
                    name: (&custom.name).into(),
                    data: (&custom.data).into(),
                });
            }
        }

        Ok(wasm_module.finish())
    }

    /// Check if the module uses instructions that require a data count section.
    fn needs_data_count_section(&self) -> bool {
        for body in &self.bodies {
            if let FuncBody::Decoded(def) = body {
                for (_span, instr) in &def.instructions {
                    match instr {
                        Instruction::MemoryInit { .. }
                        | Instruction::DataDrop { .. }
                        | Instruction::ArrayNewData { .. }
                        | Instruction::ArrayInitData { .. } => return true,
                        _ => {}
                    }
                }
            }
        }
        false
    }
}

fn encode_rec_group(types: &mut wasm_encoder::TypeSection, rec_group: &RecGroup) {
    if rec_group.is_explicit {
        let enc_subs: Vec<wasm_encoder::SubType> =
            rec_group.types.iter().map(encode_sub_type_value).collect();
        types.ty().rec(enc_subs);
    } else {
        let sub = &rec_group.types[0];
        let enc_sub = encode_sub_type_value(sub);
        types.ty().subtype(&enc_sub);
    }
}

fn encode_sub_type_value(sub: &SubType) -> wasm_encoder::SubType {
    wasm_encoder::SubType {
        is_final: sub.is_final,
        supertype_idx: sub.supertype_idx,
        composite_type: encode_composite_type(&sub.composite_type),
    }
}

fn encode_composite_type(ct: &CompositeType) -> wasm_encoder::CompositeType {
    let inner = match &ct.inner {
        CompositeInnerType::Func(f) => {
            wasm_encoder::CompositeInnerType::Func(wasm_encoder::FuncType::new(
                f.params.iter().copied().map(encode_val_type),
                f.results.iter().copied().map(encode_val_type),
            ))
        }
        CompositeInnerType::Array(a) => wasm_encoder::CompositeInnerType::Array(
            wasm_encoder::ArrayType(encode_field_type(&a.field_type)),
        ),
        CompositeInnerType::Struct(s) => {
            wasm_encoder::CompositeInnerType::Struct(wasm_encoder::StructType {
                fields: s.fields.iter().map(encode_field_type).collect(),
            })
        }
        CompositeInnerType::Cont(c) => {
            wasm_encoder::CompositeInnerType::Cont(wasm_encoder::ContType(c.type_index))
        }
    };
    wasm_encoder::CompositeType {
        inner,
        shared: ct.shared,
        descriptor: None,
        describes: None,
    }
}

fn encode_field_type(ft: &FieldType) -> wasm_encoder::FieldType {
    wasm_encoder::FieldType {
        element_type: match ft.element_type {
            StorageType::I8 => wasm_encoder::StorageType::I8,
            StorageType::I16 => wasm_encoder::StorageType::I16,
            StorageType::Val(v) => wasm_encoder::StorageType::Val(encode_val_type(v)),
        },
        mutable: ft.mutable,
    }
}

fn encode_val_type(v: wasmparser::ValType) -> wasm_encoder::ValType {
    match v {
        wasmparser::ValType::I32 => wasm_encoder::ValType::I32,
        wasmparser::ValType::I64 => wasm_encoder::ValType::I64,
        wasmparser::ValType::F32 => wasm_encoder::ValType::F32,
        wasmparser::ValType::F64 => wasm_encoder::ValType::F64,
        wasmparser::ValType::V128 => wasm_encoder::ValType::V128,
        wasmparser::ValType::Ref(r) => wasm_encoder::ValType::Ref(encode_ref_type(r)),
    }
}

fn encode_ref_type(r: wasmparser::RefType) -> wasm_encoder::RefType {
    wasm_encoder::RefType {
        nullable: r.is_nullable(),
        heap_type: encode_heap_type(r.heap_type()),
    }
}

fn encode_heap_type(h: wasmparser::HeapType) -> wasm_encoder::HeapType {
    match h {
        wasmparser::HeapType::Abstract { shared, ty } => {
            let abs = match ty {
                wasmparser::AbstractHeapType::Func => wasm_encoder::AbstractHeapType::Func,
                wasmparser::AbstractHeapType::Extern => wasm_encoder::AbstractHeapType::Extern,
                wasmparser::AbstractHeapType::Any => wasm_encoder::AbstractHeapType::Any,
                wasmparser::AbstractHeapType::None => wasm_encoder::AbstractHeapType::None,
                wasmparser::AbstractHeapType::NoExtern => wasm_encoder::AbstractHeapType::NoExtern,
                wasmparser::AbstractHeapType::NoFunc => wasm_encoder::AbstractHeapType::NoFunc,
                wasmparser::AbstractHeapType::Eq => wasm_encoder::AbstractHeapType::Eq,
                wasmparser::AbstractHeapType::Struct => wasm_encoder::AbstractHeapType::Struct,
                wasmparser::AbstractHeapType::Array => wasm_encoder::AbstractHeapType::Array,
                wasmparser::AbstractHeapType::I31 => wasm_encoder::AbstractHeapType::I31,
                wasmparser::AbstractHeapType::Exn => wasm_encoder::AbstractHeapType::Exn,
                wasmparser::AbstractHeapType::NoExn => wasm_encoder::AbstractHeapType::NoExn,
                wasmparser::AbstractHeapType::Cont => wasm_encoder::AbstractHeapType::Cont,
                wasmparser::AbstractHeapType::NoCont => wasm_encoder::AbstractHeapType::NoCont,
            };
            wasm_encoder::HeapType::Abstract { shared, ty: abs }
        }
        wasmparser::HeapType::Concrete(idx) => {
            wasm_encoder::HeapType::Concrete(idx.as_module_index().unwrap())
        }
        wasmparser::HeapType::Exact(idx) => {
            wasm_encoder::HeapType::Exact(idx.as_module_index().unwrap())
        }
    }
}

fn encode_import(imports: &mut wasm_encoder::ImportSection, import: &Import) {
    let entity = match &import.ty {
        ImportType::Func(idx) => wasm_encoder::EntityType::Function(*idx),
        ImportType::Table(t) => wasm_encoder::EntityType::Table(encode_table_type(t)),
        ImportType::Memory(m) => wasm_encoder::EntityType::Memory(encode_memory_type(m)),
        ImportType::Global(g) => wasm_encoder::EntityType::Global(encode_global_type(g)),
        ImportType::Tag(t) => wasm_encoder::EntityType::Tag(wasm_encoder::TagType {
            kind: wasm_encoder::TagKind::Exception,
            func_type_idx: t.func_type_idx,
        }),
    };
    imports.import(&import.module, &import.name, entity);
}

fn encode_table_type(t: &wasmparser::TableType) -> wasm_encoder::TableType {
    wasm_encoder::TableType {
        element_type: encode_ref_type(t.element_type),
        table64: t.table64,
        minimum: t.initial,
        maximum: t.maximum,
        shared: t.shared,
    }
}

fn encode_memory_type(m: &wasmparser::MemoryType) -> wasm_encoder::MemoryType {
    wasm_encoder::MemoryType {
        memory64: m.memory64,
        shared: m.shared,
        minimum: m.initial,
        maximum: m.maximum,
        page_size_log2: m.page_size_log2,
    }
}

fn encode_global_type(g: &wasmparser::GlobalType) -> wasm_encoder::GlobalType {
    wasm_encoder::GlobalType {
        val_type: encode_val_type(g.content_type),
        mutable: g.mutable,
        shared: g.shared,
    }
}

fn encode_export_kind(kind: ExternalKind) -> wasm_encoder::ExportKind {
    match kind {
        ExternalKind::Func => wasm_encoder::ExportKind::Func,
        ExternalKind::Table => wasm_encoder::ExportKind::Table,
        ExternalKind::Memory => wasm_encoder::ExportKind::Memory,
        ExternalKind::Global => wasm_encoder::ExportKind::Global,
        ExternalKind::Tag => wasm_encoder::ExportKind::Tag,
    }
}

fn encode_table(tables: &mut wasm_encoder::TableSection, table: &Table) {
    let ty = encode_table_type(&table.ty);
    if let Some(init) = &table.init {
        tables.table_with_init(ty, &encode_const_expr(init));
    } else {
        tables.table(ty);
    }
}

fn encode_element(elements: &mut wasm_encoder::ElementSection, element: &Element) {
    let mode = match &element.kind {
        ElementKind::Passive => wasm_encoder::ElementMode::Passive,
        ElementKind::Active {
            table_index,
            offset_expr,
        } => wasm_encoder::ElementMode::Active {
            table: *table_index,
            offset: &encode_const_expr(offset_expr),
        },
        ElementKind::Declared => wasm_encoder::ElementMode::Declared,
    };

    match &element.items {
        ElementItems::Functions(funcs) => {
            elements.segment(wasm_encoder::ElementSegment {
                mode,
                elements: wasm_encoder::Elements::Functions(std::borrow::Cow::Borrowed(funcs)),
            });
        }
        ElementItems::Expressions(ref_type, exprs) => {
            let encoded_exprs: Vec<wasm_encoder::ConstExpr> =
                exprs.iter().map(encode_const_expr).collect();
            elements.segment(wasm_encoder::ElementSegment {
                mode,
                elements: wasm_encoder::Elements::Expressions(
                    encode_ref_type(*ref_type),
                    std::borrow::Cow::Owned(encoded_exprs),
                ),
            });
        }
    }
}

fn encode_data(data_section: &mut wasm_encoder::DataSection, data: &Data) {
    match &data.kind {
        DataKind::Passive => {
            data_section.segment(wasm_encoder::DataSegment {
                mode: wasm_encoder::DataSegmentMode::Passive,
                data: data.data.iter().copied(),
            });
        }
        DataKind::Active {
            memory_index,
            offset_expr,
        } => {
            data_section.segment(wasm_encoder::DataSegment {
                mode: wasm_encoder::DataSegmentMode::Active {
                    memory_index: *memory_index,
                    offset: &encode_const_expr(offset_expr),
                },
                data: data.data.iter().copied(),
            });
        }
    }
}

fn encode_function_body(def: &FuncBodyDef) -> wasm_encoder::Function {
    // Expand locals into a flat list and re-group by type to normalize encoding
    // (strips zero-count groups and merges adjacent groups of the same type)
    let mut flat_locals: Vec<wasmparser::ValType> = Vec::new();
    for (count, ty) in &def.locals {
        for _ in 0..*count {
            flat_locals.push(*ty);
        }
    }
    let mut locals: Vec<(u32, wasm_encoder::ValType)> = Vec::new();
    for ty in &flat_locals {
        let enc_ty = encode_val_type(*ty);
        if let Some(last) = locals.last_mut() {
            if last.1 == enc_ty {
                last.0 += 1;
                continue;
            }
        }
        locals.push((1, enc_ty));
    }
    let mut func = wasm_encoder::Function::new(locals);
    for (_span, instr) in &def.instructions {
        func.instruction(&encode_instruction(instr));
    }
    func
}

fn encode_const_expr(expr: &ConstExpr) -> wasm_encoder::ConstExpr {
    let mut bytes = Vec::new();
    for op in &expr.ops {
        let enc_instr = encode_instruction(op);
        enc_instr.encode(&mut bytes);
    }
    // Note: ConstExpr::encode() adds the End byte automatically
    wasm_encoder::ConstExpr::raw(bytes)
}

fn encode_block_type(bt: wasmparser::BlockType) -> wasm_encoder::BlockType {
    match bt {
        wasmparser::BlockType::Empty => wasm_encoder::BlockType::Empty,
        wasmparser::BlockType::Type(t) => wasm_encoder::BlockType::Result(encode_val_type(t)),
        wasmparser::BlockType::FuncType(idx) => wasm_encoder::BlockType::FunctionType(idx),
    }
}

fn encode_memarg(m: &wasmparser::MemArg) -> wasm_encoder::MemArg {
    wasm_encoder::MemArg {
        offset: m.offset,
        align: m.align as u32,
        memory_index: m.memory,
    }
}

fn encode_ordering(o: &wasmparser::Ordering) -> wasm_encoder::Ordering {
    match o {
        wasmparser::Ordering::SeqCst => wasm_encoder::Ordering::SeqCst,
        wasmparser::Ordering::AcqRel => wasm_encoder::Ordering::AcqRel,
    }
}

fn encode_handle(h: &wasmparser::Handle) -> wasm_encoder::Handle {
    match h {
        wasmparser::Handle::OnLabel { tag, label } => wasm_encoder::Handle::OnLabel {
            tag: *tag,
            label: *label,
        },
        wasmparser::Handle::OnSwitch { tag } => wasm_encoder::Handle::OnSwitch { tag: *tag },
    }
}

fn encode_catch(c: &wasmparser::Catch) -> wasm_encoder::Catch {
    match c {
        wasmparser::Catch::One { tag, label } => wasm_encoder::Catch::One {
            tag: *tag,
            label: *label,
        },
        wasmparser::Catch::OneRef { tag, label } => wasm_encoder::Catch::OneRef {
            tag: *tag,
            label: *label,
        },
        wasmparser::Catch::All { label } => wasm_encoder::Catch::All { label: *label },
        wasmparser::Catch::AllRef { label } => wasm_encoder::Catch::AllRef { label: *label },
    }
}

/// Encode our owned Instruction to wasm_encoder::Instruction.
///
/// Follows the same pattern as reencode.rs: first map fields, then build.
macro_rules! encode_instruction_impl {
    ($( @$proposal:ident $op:ident $({ $($arg:ident : $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        fn encode_instruction(instr: &Instruction) -> wasm_encoder::Instruction<'static> {
            use wasm_encoder::Instruction as I;
            use std::borrow::Cow;
            match instr {
                $(
                    Instruction::$op $({ $($arg),* })? => {
                        $(
                            $(let $arg = encode_instruction_impl!(@map $arg $arg);)*
                        )?
                        encode_instruction_impl!(@build $op $($($arg)*)?)
                    }
                )*
            }
        }
    };

    // Map fields from our owned types to wasm-encoder types (by field name)
    (@map $arg:ident tag_index) => { *$arg };
    (@map $arg:ident function_index) => { *$arg };
    (@map $arg:ident table) => { *$arg };
    (@map $arg:ident table_index) => { *$arg };
    (@map $arg:ident dst_table) => { *$arg };
    (@map $arg:ident src_table) => { *$arg };
    (@map $arg:ident type_index) => { *$arg };
    (@map $arg:ident array_type_index) => { *$arg };
    (@map $arg:ident array_type_index_dst) => { *$arg };
    (@map $arg:ident array_type_index_src) => { *$arg };
    (@map $arg:ident struct_type_index) => { *$arg };
    (@map $arg:ident global_index) => { *$arg };
    (@map $arg:ident mem) => { *$arg };
    (@map $arg:ident src_mem) => { *$arg };
    (@map $arg:ident dst_mem) => { *$arg };
    (@map $arg:ident data_index) => { *$arg };
    (@map $arg:ident elem_index) => { *$arg };
    (@map $arg:ident array_data_index) => { *$arg };
    (@map $arg:ident array_elem_index) => { *$arg };
    (@map $arg:ident blockty) => { encode_block_type(*$arg) };
    (@map $arg:ident relative_depth) => { *$arg };
    (@map $arg:ident targets) => {
        (Cow::Owned($arg.targets.clone()), $arg.default)
    };
    (@map $arg:ident ty) => { encode_val_type(*$arg) };
    (@map $arg:ident tys) => { Cow::<'static, [wasm_encoder::ValType]>::Owned($arg.iter().map(|t| encode_val_type(*t)).collect()) };
    (@map $arg:ident hty) => { encode_heap_type(*$arg) };
    (@map $arg:ident from_ref_type) => { encode_ref_type(*$arg) };
    (@map $arg:ident to_ref_type) => { encode_ref_type(*$arg) };
    (@map $arg:ident memarg) => { encode_memarg($arg) };
    (@map $arg:ident ordering) => { encode_ordering($arg) };
    (@map $arg:ident local_index) => { *$arg };
    (@map $arg:ident value) => { *$arg };
    (@map $arg:ident lane) => { *$arg };
    (@map $arg:ident lanes) => { *$arg };
    (@map $arg:ident array_size) => { *$arg };
    (@map $arg:ident field_index) => { *$arg };
    (@map $arg:ident try_table) => { $arg.clone() };
    (@map $arg:ident argument_index) => { *$arg };
    (@map $arg:ident result_index) => { *$arg };
    (@map $arg:ident cont_type_index) => { *$arg };
    (@map $arg:ident resume_table) => {
        Cow::<'static, [wasm_encoder::Handle]>::Owned(
            $arg.handlers.iter().map(|h| encode_handle(h)).collect()
        )
    };

    // Build instructions - special cases first, then generic
    (@build $op:ident) => { I::$op };
    (@build BrTable $arg:ident) => { I::BrTable($arg.0, $arg.1) };
    (@build TypedSelectMulti $arg:ident) => { I::TypedSelectMulti($arg) };
    (@build I32Const $arg:ident) => { I::I32Const($arg) };
    (@build I64Const $arg:ident) => { I::I64Const($arg) };
    (@build F32Const $arg:ident) => { I::F32Const($arg.into()) };
    (@build F64Const $arg:ident) => { I::F64Const($arg.into()) };
    (@build V128Const $arg:ident) => { I::V128Const($arg.i128()) };
    (@build TryTable $table:ident) => {
        I::TryTable(
            encode_block_type($table.ty),
            Cow::Owned($table.catches.iter().map(encode_catch).collect()),
        )
    };
    (@build Resume $arg0:ident $arg1:ident) => { I::Resume { cont_type_index: $arg0, resume_table: $arg1 } };
    (@build ResumeThrow $arg0:ident $arg1:ident $arg2:ident) => {
        I::ResumeThrow { cont_type_index: $arg0, tag_index: $arg1, resume_table: $arg2 }
    };
    (@build ResumeThrowRef $arg0:ident $arg1:ident) => {
        I::ResumeThrowRef { cont_type_index: $arg0, resume_table: $arg1 }
    };
    (@build $op:ident $arg:ident) => { I::$op($arg) };
    (@build $op:ident $($arg:ident)*) => { I::$op { $($arg),* } };
}

wasmparser::for_each_operator!(encode_instruction_impl);

fn encode_name_section(wasm_module: &mut wasm_encoder::Module, names: &NameSection) {
    let mut has_names = false;
    if names.module_name.is_some()
        || !names.function_names.is_empty()
        || !names.local_names.is_empty()
        || !names.type_names.is_empty()
        || !names.table_names.is_empty()
        || !names.memory_names.is_empty()
        || !names.global_names.is_empty()
        || !names.element_names.is_empty()
        || !names.data_names.is_empty()
        || !names.tag_names.is_empty()
        || !names.field_names.is_empty()
        || !names.label_names.is_empty()
    {
        has_names = true;
    }

    if !has_names {
        return;
    }

    let mut name_section = wasm_encoder::NameSection::new();

    if let Some(ref module_name) = names.module_name {
        name_section.module(module_name);
    }

    if !names.function_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.function_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.functions(&map);
    }

    if !names.local_names.is_empty() {
        let mut indirect = wasm_encoder::IndirectNameMap::new();
        let mut func_entries: Vec<_> = names.local_names.iter().collect();
        func_entries.sort_by_key(|(k, _)| *k);
        for (func_idx, local_map) in func_entries {
            let mut map = wasm_encoder::NameMap::new();
            let mut entries: Vec<_> = local_map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            for (local_idx, name) in entries {
                map.append(*local_idx, name);
            }
            indirect.append(*func_idx, &map);
        }
        name_section.locals(&indirect);
    }

    if !names.label_names.is_empty() {
        let mut indirect = wasm_encoder::IndirectNameMap::new();
        let mut func_entries: Vec<_> = names.label_names.iter().collect();
        func_entries.sort_by_key(|(k, _)| *k);
        for (func_idx, label_map) in func_entries {
            let mut map = wasm_encoder::NameMap::new();
            let mut entries: Vec<_> = label_map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            for (label_idx, name) in entries {
                map.append(*label_idx, name);
            }
            indirect.append(*func_idx, &map);
        }
        name_section.labels(&indirect);
    }

    if !names.type_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.type_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.types(&map);
    }

    if !names.table_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.table_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.tables(&map);
    }

    if !names.memory_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.memory_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.memories(&map);
    }

    if !names.global_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.global_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.globals(&map);
    }

    if !names.element_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.element_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.elements(&map);
    }

    if !names.data_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.data_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.data(&map);
    }

    if !names.tag_names.is_empty() {
        let mut map = wasm_encoder::NameMap::new();
        let mut entries: Vec<_> = names.tag_names.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (idx, name) in entries {
            map.append(*idx, name);
        }
        name_section.tag(&map);
    }

    if !names.field_names.is_empty() {
        let mut indirect = wasm_encoder::IndirectNameMap::new();
        let mut type_entries: Vec<_> = names.field_names.iter().collect();
        type_entries.sort_by_key(|(k, _)| *k);
        for (type_idx, field_map) in type_entries {
            let mut map = wasm_encoder::NameMap::new();
            let mut entries: Vec<_> = field_map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            for (field_idx, name) in entries {
                map.append(*field_idx, name);
            }
            indirect.append(*type_idx, &map);
        }
        name_section.fields(&indirect);
    }

    wasm_module.section(&name_section);
}

/// Helper trait to encode instructions to bytes (needed for const_expr).
trait EncodeToBytes {
    fn encode(&self, bytes: &mut Vec<u8>);
}

impl EncodeToBytes for wasm_encoder::Instruction<'_> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        wasm_encoder::Encode::encode(self, bytes);
    }
}
