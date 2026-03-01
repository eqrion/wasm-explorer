use crate::ast::const_expr::ConstExpr;
use crate::ast::data::{Data, DataKind};
use crate::ast::elements::{Element, ElementItems, ElementKind};
use crate::ast::exports::ExternalKind;
use crate::ast::functions::{Func, FuncBody, FuncBodyDef};
use crate::ast::globals::Global;
use crate::ast::imports::ImportType;
use crate::ast::tags::Tag;
use crate::ast::instructions::Instruction;
use crate::ast::memories::Memory;
use crate::ast::module::{Item, ItemId, Module};
use crate::ast::tables::Table;
use crate::ast::types::{CompositeInnerType, SubType};
use crate::printer::{Printer, Style};

impl Module {
    /// Print the module to WAT text format.
    pub fn print(&self) -> String {
        let mut p = crate::printer::PlainTextPrinter::new();
        self.print_to(&mut p);
        p.into_output()
    }

    /// Print the module to WAT text format using a custom printer.
    pub fn print_to(&self, p: &mut dyn Printer) {
        print_module_impl(self, p);
    }
}

fn print_module_impl(module: &Module, p: &mut dyn Printer) {
    let ctx = PrintContext::new(module);

    // Check if module is empty
    let is_empty = module.types.is_empty()
        && module.imports.is_empty()
        && module.functions.is_empty()
        && module.tables.is_empty()
        && module.memories.is_empty()
        && module.tags.is_empty()
        && module.globals.is_empty()
        && module.exports.is_empty()
        && module.start.is_none()
        && module.elements.is_empty()
        && module.data.is_empty()
        && module.bodies.is_empty()
        && module.custom_sections.is_empty();

    p.write_str("(");
    write_keyword(p, "module");
    if let Some(ref name) = module.names.module_name {
        p.write_str(" ");
        write_name(p, name);
    }

    if is_empty {
        p.write_str(")");
        p.newline(None);
        return;
    }

    p.newline(None);
    p.indent();

    // Custom sections before anything
    print_custom_sections(&ctx, p, "before first");

    // Type section
    print_types(&ctx, p);
    print_custom_sections(&ctx, p, "after type");

    // Import section
    print_imports(&ctx, p);
    print_custom_sections(&ctx, p, "after import");

    // Function section (no output, but custom sections can follow)
    print_custom_sections(&ctx, p, "after func");

    // Table section (non-imported)
    print_tables(&ctx, p);
    print_custom_sections(&ctx, p, "after table");

    // Memory section (non-imported)
    print_memories(&ctx, p);
    print_custom_sections(&ctx, p, "after memory");

    // Tag section (non-imported)
    print_tags(&ctx, p);
    print_custom_sections(&ctx, p, "after tag");

    // Global section (non-imported)
    print_globals(&ctx, p);
    print_custom_sections(&ctx, p, "after global");

    // Export section
    print_exports(&ctx, p);
    print_custom_sections(&ctx, p, "after export");

    // Start section
    if let Some(start) = module.start {
        p.write_str("(");
        write_keyword(p, "start");
        p.write_str(" ");
        print_func_idx(&ctx, p, start);
        p.write_str(")");
        p.newline(None);
    }
    print_custom_sections(&ctx, p, "after start");

    // Element section
    print_elements(&ctx, p);
    print_custom_sections(&ctx, p, "after elem");

    // Data count is implicit
    print_custom_sections(&ctx, p, "after data count");

    // Code section (functions with bodies)
    print_functions(&ctx, p);
    print_custom_sections(&ctx, p, "after code");

    // Data section
    print_data(&ctx, p);
    print_custom_sections(&ctx, p, "after data");

    // Any custom sections without placement (e.g., at the very end)
    print_custom_sections_no_placement(&ctx, p);

    p.dedent();
    p.write_str(")");
    p.newline(None);
}

/// Context for printing a module, providing name lookups and import counts.
pub struct PrintContext<'a> {
    /// The module being printed.
    pub module: &'a Module,
    num_imported_funcs: u32,
    num_imported_tables: u32,
    num_imported_memories: u32,
    num_imported_globals: u32,
    num_imported_tags: u32,
}

impl<'a> PrintContext<'a> {
    /// Create a new print context for the given module.
    pub fn new(module: &'a Module) -> Self {
        let mut num_imported_funcs = 0u32;
        let mut num_imported_tables = 0u32;
        let mut num_imported_memories = 0u32;
        let mut num_imported_globals = 0u32;
        let mut num_imported_tags = 0u32;
        for imp in &module.imports {
            match &imp.ty {
                ImportType::Func(_) => num_imported_funcs += 1,
                ImportType::Table(_) => num_imported_tables += 1,
                ImportType::Memory(_) => num_imported_memories += 1,
                ImportType::Global(_) => num_imported_globals += 1,
                ImportType::Tag(_) => num_imported_tags += 1,
            }
        }
        PrintContext {
            module,
            num_imported_funcs,
            num_imported_tables,
            num_imported_memories,
            num_imported_globals,
            num_imported_tags,
        }
    }

    fn func_name(&self, idx: u32) -> Option<&str> {
        self.module
            .names
            .function_names
            .get(&idx)
            .map(|s| s.as_str())
    }

    fn type_name(&self, idx: u32) -> Option<&str> {
        self.module.names.type_names.get(&idx).map(|s| s.as_str())
    }

    fn table_name(&self, idx: u32) -> Option<&str> {
        self.module.names.table_names.get(&idx).map(|s| s.as_str())
    }

    fn memory_name(&self, idx: u32) -> Option<&str> {
        self.module.names.memory_names.get(&idx).map(|s| s.as_str())
    }

    fn global_name(&self, idx: u32) -> Option<&str> {
        self.module.names.global_names.get(&idx).map(|s| s.as_str())
    }

    fn tag_name(&self, idx: u32) -> Option<&str> {
        self.module.names.tag_names.get(&idx).map(|s| s.as_str())
    }

    fn data_name(&self, idx: u32) -> Option<&str> {
        self.module.names.data_names.get(&idx).map(|s| s.as_str())
    }

    fn elem_name(&self, idx: u32) -> Option<&str> {
        self.module
            .names
            .element_names
            .get(&idx)
            .map(|s| s.as_str())
    }

    fn field_name(&self, type_idx: u32, field_idx: u32) -> Option<&str> {
        self.module
            .names
            .field_names
            .get(&type_idx)
            .and_then(|m| m.get(&field_idx))
            .map(|s| s.as_str())
    }
}

fn print_name_and_index(p: &mut dyn Printer, name: Option<&str>, idx: u32) {
    if let Some(name) = name {
        p.write_str(" ");
        write_name(p, name);
    }
    p.push_style(Style::Comment);
    p.write_str(" (;");
    p.write_str(&idx.to_string());
    p.write_str(";)");
    p.pop_style();
}

fn print_idx(p: &mut dyn Printer, name: Option<&str>, idx: u32) {
    if let Some(name) = name {
        write_name(p, name);
    } else {
        p.push_style(Style::Literal);
        p.write_str(&idx.to_string());
        p.pop_style();
    }
}

fn write_keyword(p: &mut dyn Printer, s: &str) {
    p.push_style(Style::Keyword);
    p.write_str(s);
    p.pop_style();
}

fn write_type_kw(p: &mut dyn Printer, s: &str) {
    p.push_style(Style::Type);
    p.write_str(s);
    p.pop_style();
}

fn write_name(p: &mut dyn Printer, name: &str) {
    p.push_style(Style::Name);
    p.write_str("$");
    print_id_name(p, name);
    p.pop_style();
}

/// Print a WAT identifier name, quoting if it contains special characters.
fn print_id_name(p: &mut dyn Printer, name: &str) {
    // Check if name needs quoting (contains non-idchar characters)
    let needs_quote = name.is_empty()
        || name.bytes().any(|b| {
            !matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z'
            | b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.'
            | b'/' | b':' | b'<' | b'=' | b'>' | b'?' | b'@' | b'\\' | b'^' | b'_'
            | b'`' | b'|' | b'~')
        });
    if needs_quote {
        p.write_str("\"");
        for ch in name.chars() {
            match ch {
                '"' => p.write_str("\\\""),
                '\\' => p.write_str("\\\\"),
                c if c.is_ascii_graphic() || c == ' ' => {
                    let mut buf = [0u8; 4];
                    p.write_str(c.encode_utf8(&mut buf));
                }
                c => {
                    p.write_str(&format!("\\u{{{:x}}}", c as u32));
                }
            }
        }
        p.write_str("\"");
    } else {
        p.write_str(name);
    }
}

fn print_func_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Func(idx));
    print_idx(p, ctx.func_name(idx), idx);
    p.end_xref();
}

fn print_type_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Type(idx));
    print_idx(p, ctx.type_name(idx), idx);
    p.end_xref();
}

fn print_table_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Table(idx));
    print_idx(p, ctx.table_name(idx), idx);
    p.end_xref();
}

fn print_memory_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Memory(idx));
    print_idx(p, ctx.memory_name(idx), idx);
    p.end_xref();
}

fn print_global_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Global(idx));
    print_idx(p, ctx.global_name(idx), idx);
    p.end_xref();
}

fn print_tag_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Tag(idx));
    print_idx(p, ctx.tag_name(idx), idx);
    p.end_xref();
}

fn print_elem_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Element(idx));
    print_idx(p, ctx.elem_name(idx), idx);
    p.end_xref();
}

fn print_data_idx(ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
    p.begin_xref(ItemId::Data(idx));
    print_idx(p, ctx.data_name(idx), idx);
    p.end_xref();
}

fn print_val_type(p: &mut dyn Printer, ty: wasmparser::ValType) {
    print_val_type_ctx(None, p, ty);
}

fn print_val_type_ctx(ctx: Option<&PrintContext>, p: &mut dyn Printer, ty: wasmparser::ValType) {
    match ty {
        wasmparser::ValType::I32 => write_type_kw(p, "i32"),
        wasmparser::ValType::I64 => write_type_kw(p, "i64"),
        wasmparser::ValType::F32 => write_type_kw(p, "f32"),
        wasmparser::ValType::F64 => write_type_kw(p, "f64"),
        wasmparser::ValType::V128 => write_type_kw(p, "v128"),
        wasmparser::ValType::Ref(r) => print_ref_type_ctx(ctx, p, r),
    }
}

fn print_ref_type_ctx(ctx: Option<&PrintContext>, p: &mut dyn Printer, r: wasmparser::RefType) {
    let heap = r.heap_type();
    let nullable = r.is_nullable();

    // Shorthand forms
    if let wasmparser::HeapType::Abstract { shared: false, ty } = heap {
        match (ty, nullable) {
            (wasmparser::AbstractHeapType::Func, true) => {
                write_type_kw(p, "funcref");
                return;
            }
            (wasmparser::AbstractHeapType::Extern, true) => {
                write_type_kw(p, "externref");
                return;
            }
            (wasmparser::AbstractHeapType::Exn, true) => {
                write_type_kw(p, "exnref");
                return;
            }
            (wasmparser::AbstractHeapType::Any, true) => {
                write_type_kw(p, "anyref");
                return;
            }
            (wasmparser::AbstractHeapType::Eq, true) => {
                write_type_kw(p, "eqref");
                return;
            }
            (wasmparser::AbstractHeapType::Struct, true) => {
                write_type_kw(p, "structref");
                return;
            }
            (wasmparser::AbstractHeapType::Array, true) => {
                write_type_kw(p, "arrayref");
                return;
            }
            (wasmparser::AbstractHeapType::I31, true) => {
                write_type_kw(p, "i31ref");
                return;
            }
            (wasmparser::AbstractHeapType::None, true) => {
                write_type_kw(p, "nullref");
                return;
            }
            (wasmparser::AbstractHeapType::NoFunc, true) => {
                write_type_kw(p, "nullfuncref");
                return;
            }
            (wasmparser::AbstractHeapType::NoExtern, true) => {
                write_type_kw(p, "nullexternref");
                return;
            }
            (wasmparser::AbstractHeapType::NoExn, true) => {
                write_type_kw(p, "nullexnref");
                return;
            }
            _ => {}
        }
    }

    // Long form: (ref null? heap_type)
    p.write_str("(");
    write_type_kw(p, "ref");
    p.write_str(" ");
    if nullable {
        write_type_kw(p, "null");
        p.write_str(" ");
    }
    print_heap_type_ctx(ctx, p, heap);
    p.write_str(")");
}

fn print_heap_type_ctx(ctx: Option<&PrintContext>, p: &mut dyn Printer, h: wasmparser::HeapType) {
    match h {
        wasmparser::HeapType::Abstract { shared: _, ty } => match ty {
            wasmparser::AbstractHeapType::Func => write_type_kw(p, "func"),
            wasmparser::AbstractHeapType::Extern => write_type_kw(p, "extern"),
            wasmparser::AbstractHeapType::Any => write_type_kw(p, "any"),
            wasmparser::AbstractHeapType::None => write_type_kw(p, "none"),
            wasmparser::AbstractHeapType::NoExtern => write_type_kw(p, "noextern"),
            wasmparser::AbstractHeapType::NoFunc => write_type_kw(p, "nofunc"),
            wasmparser::AbstractHeapType::Eq => write_type_kw(p, "eq"),
            wasmparser::AbstractHeapType::Struct => write_type_kw(p, "struct"),
            wasmparser::AbstractHeapType::Array => write_type_kw(p, "array"),
            wasmparser::AbstractHeapType::I31 => write_type_kw(p, "i31"),
            wasmparser::AbstractHeapType::Exn => write_type_kw(p, "exn"),
            wasmparser::AbstractHeapType::NoExn => write_type_kw(p, "noexn"),
            wasmparser::AbstractHeapType::Cont => write_type_kw(p, "cont"),
            wasmparser::AbstractHeapType::NoCont => write_type_kw(p, "nocont"),
        },
        wasmparser::HeapType::Concrete(idx) => {
            let i = idx.as_module_index().unwrap();
            let name = ctx.and_then(|c| c.type_name(i));
            p.begin_xref(ItemId::Type(i));
            print_idx(p, name, i);
            p.end_xref();
        }
        wasmparser::HeapType::Exact(idx) => {
            let i = idx.as_module_index().unwrap();
            let name = ctx.and_then(|c| c.type_name(i));
            p.write_str("(");
            write_type_kw(p, "exact");
            p.write_str(" ");
            p.begin_xref(ItemId::Type(i));
            print_idx(p, name, i);
            p.end_xref();
            p.write_str(")");
        }
    }
}

// ---- Type section ----

fn print_types(ctx: &PrintContext, p: &mut dyn Printer) {
    let mut type_idx = 0u32;
    for rec_group in &ctx.module.types {
        if rec_group.is_explicit {
            let offset = Some(rec_group.span.offset);
            if rec_group.types.is_empty() {
                p.write_str("(");
                write_keyword(p, "rec");
                p.write_str(")");
                p.newline(offset);
            } else {
                p.write_str("(");
                write_keyword(p, "rec");
                p.newline(offset);
                p.indent();
                for sub in &rec_group.types {
                    sub.print(ctx, p, type_idx);
                    type_idx += 1;
                }
                p.dedent();
                p.write_str(")");
                p.newline(offset);
            }
        } else if let Some(sub) = rec_group.types.first() {
            sub.print(ctx, p, type_idx);
            type_idx += 1;
        }
    }
}

impl Item for SubType {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        let sub = self;
    p.write_str("(");
    write_keyword(p, "type");
    print_name_and_index(p, ctx.type_name(idx), idx);
    p.write_str(" ");

    // Sub type wrapper if non-final or has supertype
    let needs_sub = !sub.is_final || sub.supertype_idx.is_some();
    if needs_sub {
        p.write_str("(");
        write_keyword(p, "sub");
        p.write_str(" ");
        if sub.is_final {
            write_keyword(p, "final");
            p.write_str(" ");
        }
        if let Some(sup) = sub.supertype_idx {
            print_type_idx(ctx, p, sup);
            p.write_str(" ");
        }
    }

    let shared = sub.composite_type.shared;
    if shared {
        p.write_str("(");
        write_keyword(p, "shared");
        p.write_str(" ");
    }

    match &sub.composite_type.inner {
        CompositeInnerType::Func(f) => {
            p.write_str("(");
            write_keyword(p, "func");
            if !f.params.is_empty() {
                p.write_str(" (");
                write_keyword(p, "param");
                for param in &f.params {
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, *param);
                }
                p.write_str(")");
            }
            if !f.results.is_empty() {
                p.write_str(" (");
                write_keyword(p, "result");
                for result in &f.results {
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, *result);
                }
                p.write_str(")");
            }
            p.write_str(")");
        }
        CompositeInnerType::Array(a) => {
            p.write_str("(");
            write_keyword(p, "array");
            p.write_str(" ");
            print_field_type_ctx(ctx, p, &a.field_type);
            p.write_str(")");
        }
        CompositeInnerType::Struct(s) => {
            p.write_str("(");
            write_keyword(p, "struct");
            let field_names = ctx.module.names.field_names.get(&idx);
            for (fi, field) in s.fields.iter().enumerate() {
                p.write_str(" (");
                write_keyword(p, "field");
                if let Some(name) = field_names.and_then(|m| m.get(&(fi as u32))) {
                    p.write_str(" ");
                    write_name(p, name);
                }
                p.write_str(" ");
                print_field_type_ctx(ctx, p, field);
                p.write_str(")");
            }
            p.write_str(")");
        }
        CompositeInnerType::Cont(c) => {
            p.write_str("(");
            write_keyword(p, "cont");
            p.write_str(" ");
            print_type_idx(ctx, p, c.type_index);
            p.write_str(")");
        }
    }

    if shared {
        p.write_str(")");
    }
    if needs_sub {
        p.write_str(")");
    }
    p.write_str(")");
    p.newline(None);
    }
}

fn print_field_type_ctx(
    ctx: &PrintContext,
    p: &mut dyn Printer,
    ft: &crate::ast::types::FieldType,
) {
    if ft.mutable {
        p.write_str("(");
        write_keyword(p, "mut");
        p.write_str(" ");
        print_storage_type_ctx(Some(ctx), p, ft.element_type);
        p.write_str(")");
    } else {
        print_storage_type_ctx(Some(ctx), p, ft.element_type);
    }
}

fn print_storage_type_ctx(
    ctx: Option<&PrintContext>,
    p: &mut dyn Printer,
    st: crate::ast::types::StorageType,
) {
    match st {
        crate::ast::types::StorageType::I8 => write_type_kw(p, "i8"),
        crate::ast::types::StorageType::I16 => write_type_kw(p, "i16"),
        crate::ast::types::StorageType::Val(v) => print_val_type_ctx(ctx, p, v),
    }
}

// ---- Import section ----

fn print_imports(ctx: &PrintContext, p: &mut dyn Printer) {
    let mut func_idx = 0u32;
    let mut table_idx = 0u32;
    let mut memory_idx = 0u32;
    let mut global_idx = 0u32;
    let mut tag_idx = 0u32;

    for import in &ctx.module.imports {
        p.write_str("(");
        write_keyword(p, "import");
        p.write_str(" ");
        print_wat_str(p, &import.module);
        p.write_str(" ");
        print_wat_str(p, &import.name);
        p.write_str(" ");

        match &import.ty {
            ImportType::Func(type_idx) => {
                p.write_str("(");
                write_keyword(p, "func");
                print_name_and_index(p, ctx.func_name(func_idx), func_idx);
                p.write_str(" (");
                write_keyword(p, "type");
                p.write_str(" ");
                print_type_idx(ctx, p, *type_idx);
                p.write_str(")");
                p.write_str(")");
                func_idx += 1;
            }
            ImportType::Table(t) => {
                p.write_str("(");
                write_keyword(p, "table");
                print_name_and_index(p, ctx.table_name(table_idx), table_idx);
                p.write_str(" ");
                print_table_type_inline(Some(ctx), p, t);
                p.write_str(")");
                table_idx += 1;
            }
            ImportType::Memory(m) => {
                p.write_str("(");
                write_keyword(p, "memory");
                print_name_and_index(p, ctx.memory_name(memory_idx), memory_idx);
                p.write_str(" ");
                print_memory_type_inline(p, m);
                p.write_str(")");
                memory_idx += 1;
            }
            ImportType::Global(g) => {
                p.write_str("(");
                write_keyword(p, "global");
                print_name_and_index(p, ctx.global_name(global_idx), global_idx);
                p.write_str(" ");
                print_global_type(Some(ctx), p, g);
                p.write_str(")");
                global_idx += 1;
            }
            ImportType::Tag(t) => {
                p.write_str("(");
                write_keyword(p, "tag");
                print_name_and_index(p, ctx.tag_name(tag_idx), tag_idx);
                p.write_str(" (");
                write_keyword(p, "type");
                p.write_str(" ");
                print_type_idx(ctx, p, t.func_type_idx);
                p.write_str(")");
                // Expand params/results
                if let Some(ft) = get_func_type(ctx, t.func_type_idx) {
                    if !ft.params.is_empty() {
                        p.write_str(" (");
                        write_keyword(p, "param");
                        for param in &ft.params {
                            p.write_str(" ");
                            print_val_type_ctx(Some(ctx), p, *param);
                        }
                        p.write_str(")");
                    }
                    if !ft.results.is_empty() {
                        p.write_str(" (");
                        write_keyword(p, "result");
                        for result in &ft.results {
                            p.write_str(" ");
                            print_val_type_ctx(Some(ctx), p, *result);
                        }
                        p.write_str(")");
                    }
                }
                p.write_str(")");
                tag_idx += 1;
            }
        }
        p.write_str(")");
        p.newline(Some(import.span.offset));
    }
}

fn print_table_type_inline(
    ctx: Option<&PrintContext>,
    p: &mut dyn Printer,
    t: &wasmparser::TableType,
) {
    if t.table64 {
        write_type_kw(p, "i64");
        p.write_str(" ");
    }
    p.push_style(Style::Literal);
    p.write_str(&t.initial.to_string());
    p.pop_style();
    if let Some(max) = t.maximum {
        p.write_str(" ");
        p.push_style(Style::Literal);
        p.write_str(&max.to_string());
        p.pop_style();
    }
    p.write_str(" ");
    print_ref_type_ctx(ctx, p, t.element_type);
}

fn print_memory_type_inline(p: &mut dyn Printer, m: &wasmparser::MemoryType) {
    if m.memory64 {
        write_type_kw(p, "i64");
        p.write_str(" ");
    }
    p.push_style(Style::Literal);
    p.write_str(&m.initial.to_string());
    p.pop_style();
    if let Some(max) = m.maximum {
        p.write_str(" ");
        p.push_style(Style::Literal);
        p.write_str(&max.to_string());
        p.pop_style();
    }
    if m.shared {
        p.write_str(" ");
        write_keyword(p, "shared");
    }
}

fn print_global_type(ctx: Option<&PrintContext>, p: &mut dyn Printer, g: &wasmparser::GlobalType) {
    if g.mutable {
        p.write_str("(");
        write_keyword(p, "mut");
        p.write_str(" ");
        print_val_type_ctx(ctx, p, g.content_type);
        p.write_str(")");
    } else {
        print_val_type_ctx(ctx, p, g.content_type);
    }
}

impl Item for Table {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        p.write_str("(");
        write_keyword(p, "table");
        print_name_and_index(p, ctx.table_name(idx), idx);
        p.write_str(" ");
        print_table_type_inline(Some(ctx), p, &self.ty);
        if let Some(ref init) = self.init {
            p.write_str(" ");
            print_const_expr_inline(ctx, p, init);
        }
        p.write_str(")");
        p.newline(Some(self.span.offset));
    }
}

impl Item for Memory {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        p.write_str("(");
        write_keyword(p, "memory");
        print_name_and_index(p, ctx.memory_name(idx), idx);
        p.write_str(" ");
        print_memory_type_inline(p, &self.ty);
        p.write_str(")");
        p.newline(Some(self.span.offset));
    }
}

impl Item for Tag {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        p.write_str("(");
        write_keyword(p, "tag");
        print_name_and_index(p, ctx.tag_name(idx), idx);
        p.write_str(" (");
        write_keyword(p, "type");
        p.write_str(" ");
        print_type_idx(ctx, p, self.ty.func_type_idx);
        p.write_str(")");
        // Expand params/results
        if let Some(ft) = get_func_type(ctx, self.ty.func_type_idx) {
            if !ft.params.is_empty() {
                p.write_str(" (");
                write_keyword(p, "param");
                for param in &ft.params {
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, *param);
                }
                p.write_str(")");
            }
            if !ft.results.is_empty() {
                p.write_str(" (");
                write_keyword(p, "result");
                for result in &ft.results {
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, *result);
                }
                p.write_str(")");
            }
        }
        p.write_str(")");
        p.newline(Some(self.span.offset));
    }
}

impl Item for Global {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        p.write_str("(");
        write_keyword(p, "global");
        print_name_and_index(p, ctx.global_name(idx), idx);
        p.write_str(" ");
        print_global_type(Some(ctx), p, &self.ty);
        p.write_str(" ");
        print_const_expr_inline(ctx, p, &self.init_expr);
        p.write_str(")");
        p.newline(Some(self.span.offset));
    }
}

// ---- Table section ----

fn print_tables(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, table) in ctx.module.tables.iter().enumerate() {
        table.print(ctx, p, ctx.num_imported_tables + i as u32);
    }
}

// ---- Memory section ----

fn print_memories(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, memory) in ctx.module.memories.iter().enumerate() {
        memory.print(ctx, p, ctx.num_imported_memories + i as u32);
    }
}

// ---- Tag section ----

fn print_tags(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, tag) in ctx.module.tags.iter().enumerate() {
        tag.print(ctx, p, ctx.num_imported_tags + i as u32);
    }
}

// ---- Global section ----

fn print_globals(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, global) in ctx.module.globals.iter().enumerate() {
        global.print(ctx, p, ctx.num_imported_globals + i as u32);
    }
}

// ---- Export section ----

fn print_exports(ctx: &PrintContext, p: &mut dyn Printer) {
    for export in &ctx.module.exports {
        p.write_str("(");
        write_keyword(p, "export");
        p.write_str(" ");
        print_wat_str(p, &export.name);
        p.write_str(" (");
        match export.kind {
            ExternalKind::Func => {
                write_keyword(p, "func");
                p.write_str(" ");
                print_func_idx(ctx, p, export.index);
            }
            ExternalKind::Table => {
                write_keyword(p, "table");
                p.write_str(" ");
                print_table_idx(ctx, p, export.index);
            }
            ExternalKind::Memory => {
                write_keyword(p, "memory");
                p.write_str(" ");
                print_memory_idx(ctx, p, export.index);
            }
            ExternalKind::Global => {
                write_keyword(p, "global");
                p.write_str(" ");
                print_global_idx(ctx, p, export.index);
            }
            ExternalKind::Tag => {
                write_keyword(p, "tag");
                p.write_str(" ");
                p.push_style(Style::Literal);
                p.write_str(&export.index.to_string());
                p.pop_style();
            }
        }
        p.write_str("))");
        p.newline(Some(export.span.offset));
    }
}

impl Item for Element {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        let elem = self;
    p.write_str("(");
    write_keyword(p, "elem");
    print_name_and_index(p, ctx.elem_name(idx), idx);

    match &elem.kind {
        ElementKind::Active {
            table_index,
            offset_expr,
        } => {
            if let Some(table_idx) = table_index {
                p.write_str(" (");
                write_keyword(p, "table");
                p.write_str(" ");
                print_table_idx(ctx, p, *table_idx);
                p.write_str(")");
            }
            p.write_str(" ");
            print_const_expr_offset(ctx, p, offset_expr);
        }
        ElementKind::Passive => {}
        ElementKind::Declared => {
            p.write_str(" ");
            write_keyword(p, "declare");
        }
    }

    match &elem.items {
        ElementItems::Functions(funcs) => {
            p.write_str(" ");
            write_keyword(p, "func");
            for func_idx in funcs {
                p.write_str(" ");
                print_func_idx(ctx, p, *func_idx);
            }
        }
        ElementItems::Expressions(ref_type, exprs) => {
            p.write_str(" ");
            print_ref_type_ctx(Some(ctx), p, *ref_type);
            for expr in exprs {
                p.write_str(" ");
                print_const_expr_item(ctx, p, expr);
            }
        }
    }

    p.write_str(")");
    p.newline(Some(self.span.offset));
    }
}

// ---- Element section ----

fn print_elements(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, elem) in ctx.module.elements.iter().enumerate() {
        elem.print(ctx, p, i as u32);
    }
}

// ---- Function section (with bodies) ----

impl Item for Func {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        let body_index = (idx - ctx.num_imported_funcs) as usize;
        let body = ctx.module.bodies.get(body_index);

        p.write_str("(");
        write_keyword(p, "func");
        print_name_and_index(p, ctx.func_name(idx), idx);
        p.write_str(" (");
        write_keyword(p, "type");
        p.write_str(" ");
        print_type_idx(ctx, p, self.type_index);
        p.write_str(")");

        // Print expanded params and results from the type
        if let Some(func_type) = get_func_type(ctx, self.type_index) {
            // Params - use local names if available; group all consecutive unnamed params
            let local_names = ctx.module.names.local_names.get(&idx);
            let params = &func_type.params;
            let mut i = 0u32;
            while (i as usize) < params.len() {
                let name = local_names.and_then(|m| m.get(&i));
                if let Some(name) = name {
                    p.write_str(" (");
                    write_keyword(p, "param");
                    p.write_str(" ");
                    write_name(p, name);
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, params[i as usize]);
                    p.write_str(")");
                    i += 1;
                } else {
                    // Group all consecutive unnamed params together
                    p.write_str(" (");
                    write_keyword(p, "param");
                    while (i as usize) < params.len()
                        && local_names.and_then(|m| m.get(&i)).is_none()
                    {
                        p.write_str(" ");
                        print_val_type_ctx(Some(ctx), p, params[i as usize]);
                        i += 1;
                    }
                    p.write_str(")");
                }
            }
            if !func_type.results.is_empty() {
                p.write_str(" (");
                write_keyword(p, "result");
                for result in &func_type.results {
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, *result);
                }
                p.write_str(")");
            }
        }

        match body {
            Some(FuncBody::Decoded(def)) if has_body_content(def) => {
                p.newline(Some(self.span.offset));
                p.indent();
                print_function_body(ctx, p, def, idx);
                p.dedent();
                p.write_str(")");
            }
            _ => {
                p.write_str(")");
            }
        }
        p.newline(Some(self.span.offset));
    }
}

fn print_functions(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, func) in ctx.module.functions.iter().enumerate() {
        func.print(ctx, p, ctx.num_imported_funcs + i as u32);
    }
}

fn has_body_content(def: &FuncBodyDef) -> bool {
    // A body has content if it has locals or instructions beyond just End
    if !def.locals.is_empty() {
        return true;
    }
    // Check if there are non-End instructions
    def.instructions
        .iter()
        .any(|(_, instr)| !matches!(instr, Instruction::End))
}

fn get_func_type<'a>(
    ctx: &'a PrintContext,
    type_index: u32,
) -> Option<&'a crate::ast::types::FuncType> {
    let mut idx = 0u32;
    for rec_group in &ctx.module.types {
        for sub in &rec_group.types {
            if idx == type_index {
                if let CompositeInnerType::Func(f) = &sub.composite_type.inner {
                    return Some(f);
                }
                return None;
            }
            idx += 1;
        }
    }
    None
}

fn print_function_body(ctx: &PrintContext, p: &mut dyn Printer, def: &FuncBodyDef, func_idx: u32) {
    let func_type = ctx
        .module
        .functions
        .get((func_idx - ctx.num_imported_funcs) as usize)
        .and_then(|f| get_func_type(ctx, f.type_index));
    let num_params = func_type.map(|f| f.params.len() as u32).unwrap_or(0);
    let local_names = ctx.module.names.local_names.get(&func_idx);

    // Print locals - all on one line; group consecutive unnamed locals of same type
    // First, expand all locals into a flat list of (index, type) pairs
    let mut all_locals: Vec<(u32, wasmparser::ValType)> = Vec::new();
    {
        let mut idx = num_params;
        for (count, val_type) in &def.locals {
            for _ in 0..*count {
                all_locals.push((idx, *val_type));
                idx += 1;
            }
        }
    }

    if !all_locals.is_empty() {
        let mut i = 0;
        while i < all_locals.len() {
            let (idx, ty) = all_locals[i];
            let name = local_names.and_then(|m| m.get(&idx));
            if let Some(name) = name {
                p.write_str("(");
                write_keyword(p, "local");
                p.write_str(" ");
                write_name(p, name);
                p.write_str(" ");
                print_val_type_ctx(Some(ctx), p, ty);
                p.write_str(")");
                i += 1;
            } else {
                // Group ALL consecutive unnamed locals together (regardless of type)
                p.write_str("(");
                write_keyword(p, "local");
                while i < all_locals.len() {
                    let (next_idx, next_ty) = all_locals[i];
                    if local_names.and_then(|m| m.get(&next_idx)).is_some() {
                        break;
                    }
                    p.write_str(" ");
                    print_val_type_ctx(Some(ctx), p, next_ty);
                    i += 1;
                }
                p.write_str(")");
            }
            if i < all_locals.len() {
                p.write_str(" ");
            }
        }
        p.newline(None);
    }

    // Print instructions (skip the final End — it's implicit in the closing paren)
    let num_instrs = def.instructions.len();
    // label_indices tracks the sequential label index for each nesting level
    // The function body itself (depth 0) has no label in the name section, so we push u32::MAX as sentinel
    let mut label_indices: Vec<u32> = vec![u32::MAX]; // function body = @0
    let mut label_count = 0u32;
    for (i, (span, instr)) in def.instructions.iter().enumerate() {
        if i == num_instrs - 1 {
            if let Instruction::End = instr {
                break;
            }
        }
        print_instruction(ctx, p, instr, func_idx, &mut label_indices, &mut label_count);
        p.newline(Some(span.offset));
    }
}

// ---- Custom sections ----

fn print_custom_sections(ctx: &PrintContext, p: &mut dyn Printer, placement: &str) {
    for section in &ctx.module.custom_sections {
        if section.placement.as_deref() == Some(placement) {
            print_custom_section(p, section, Some(placement));
        }
    }
}

fn print_custom_sections_no_placement(ctx: &PrintContext, p: &mut dyn Printer) {
    for section in &ctx.module.custom_sections {
        if section.placement.is_none() {
            print_custom_section(p, section, None);
        }
    }
}

fn print_custom_section(
    p: &mut dyn Printer,
    section: &crate::ast::custom::CustomSection,
    placement: Option<&str>,
) {
    p.write_str("(");
    write_keyword(p, "@custom");
    p.write_str(" ");
    // Name is a UTF-8 string, use Unicode escaping (\u{N})
    print_wat_str(p, &section.name);
    if let Some(place) = placement {
        p.write_str(" (");
        p.write_str(place);
        p.write_str(")");
    }
    p.write_str(" ");
    // Data is raw bytes, use hex escaping (\xx)
    print_wat_bytes(p, &section.data);
    p.write_str(")");
    p.newline(Some(section.span.offset));
}

/// Print a UTF-8 string in WAT format with Unicode escaping (\u{N})
fn print_wat_str(p: &mut dyn Printer, s: &str) {
    p.push_style(Style::Literal);
    p.write_str("\"");
    for c in s.chars() {
        let v = c as u32;
        if (0x20..0x7f).contains(&v) && c != '"' && c != '\\' {
            let mut buf = [0u8; 4];
            p.write_str(c.encode_utf8(&mut buf));
        } else {
            p.write_str(&format!("\\u{{{:x}}}", v));
        }
    }
    p.write_str("\"");
    p.pop_style();
}

/// Print raw bytes in WAT format with hex escaping (\xx)
fn print_wat_bytes(p: &mut dyn Printer, bytes: &[u8]) {
    p.push_style(Style::Literal);
    p.write_str("\"");
    for &byte in bytes {
        if (0x20..0x7f).contains(&byte) && byte != b'"' && byte != b'\\' {
            let buf = [byte];
            p.write_str(std::str::from_utf8(&buf).unwrap());
        } else {
            p.write_str(&format!("\\{:02x}", byte));
        }
    }
    p.write_str("\"");
    p.pop_style();
}

impl Item for Data {
    fn print(&self, ctx: &PrintContext, p: &mut dyn Printer, idx: u32) {
        p.write_str("(");
        write_keyword(p, "data");
        print_name_and_index(p, ctx.data_name(idx), idx);

        match &self.kind {
            DataKind::Active {
                memory_index,
                offset_expr,
            } => {
                if *memory_index != 0 {
                    p.write_str(" (");
                    write_keyword(p, "memory");
                    p.write_str(" ");
                    print_memory_idx(ctx, p, *memory_index);
                    p.write_str(")");
                }
                p.write_str(" ");
                print_const_expr_offset(ctx, p, offset_expr);
            }
            DataKind::Passive => {}
        }

        p.write_str(" ");
        print_wat_bytes(p, &self.data);
        p.write_str(")");
        p.newline(Some(self.span.offset));
    }
}

fn print_data(ctx: &PrintContext, p: &mut dyn Printer) {
    for (i, data) in ctx.module.data.iter().enumerate() {
        data.print(ctx, p, i as u32);
    }
}

// ---- Const expressions ----

fn print_const_expr_inline(ctx: &PrintContext, p: &mut dyn Printer, expr: &ConstExpr) {
    let mut labels = Vec::new();
    let mut lc = 0u32;
    for (i, op) in expr.ops.iter().enumerate() {
        if i > 0 {
            p.write_str(" ");
        }
        print_instruction(ctx, p, op, 0, &mut labels, &mut lc);
    }
}

/// Print a const expr for element segment expressions.
/// Single-instruction: (instruction) — same as offset format.
/// Multi-instruction: (item instr1 instr2 ...) — uses "item" instead of "offset".
fn print_const_expr_item(ctx: &PrintContext, p: &mut dyn Printer, expr: &ConstExpr) {
    let ops = &expr.ops;
    if ops.len() == 1 {
        let mut labels = Vec::new();
        let mut lc = 0u32;
        p.write_str("(");
        print_instruction(ctx, p, &ops[0], 0, &mut labels, &mut lc);
        p.write_str(")");
    } else {
        p.write_str("(");
        write_keyword(p, "item");
        p.write_str(" ");
        print_const_expr_inline(ctx, p, expr);
        p.write_str(")");
    }
}

/// Print a const expr wrapped in (offset ...) for data/element segments
fn print_const_expr_offset(ctx: &PrintContext, p: &mut dyn Printer, expr: &ConstExpr) {
    let ops = &expr.ops;
    if ops.len() == 1 {
        let mut labels = Vec::new();
        let mut lc = 0u32;
        p.write_str("(");
        print_instruction(ctx, p, &ops[0], 0, &mut labels, &mut lc);
        p.write_str(")");
    } else {
        p.write_str("(");
        write_keyword(p, "offset");
        p.write_str(" ");
        print_const_expr_inline(ctx, p, expr);
        p.write_str(")");
    }
}

// ---- Instructions ----

fn print_instruction(
    ctx: &PrintContext,
    p: &mut dyn Printer,
    instr: &Instruction,
    func_idx: u32,
    label_indices: &mut Vec<u32>,
    label_count: &mut u32,
) {
    let cur_depth = label_indices.len() as u32;

    // Block-like instructions manage their own indentation and newlines.
    match instr {
        Instruction::Block { blockty }
        | Instruction::Loop { blockty }
        | Instruction::If { blockty } => {
            let name = match instr {
                Instruction::Block { .. } => "block",
                Instruction::Loop { .. } => "loop",
                _ => "if",
            };
            write_keyword(p, name);
            print_block_label_and_type(ctx, p, func_idx, *blockty, cur_depth, *label_count);
            label_indices.push(*label_count);
            *label_count += 1;
            p.indent();
            return;
        }
        Instruction::End => {
            label_indices.pop();
            p.dedent();
            write_keyword(p, "end");
            return;
        }
        Instruction::Else => {
            p.dedent();
            write_keyword(p, "else");
            p.indent();
            return;
        }
        Instruction::TryTable { try_table } => {
            write_keyword(p, "try_table");
            let label_names = ctx.module.names.label_names.get(&func_idx);
            let label_name = label_names.and_then(|m| m.get(label_count));
            if let Some(name) = label_name {
                p.write_str(" ");
                write_name(p, name);
            }
            print_blocktype(ctx, p, try_table.ty);
            for catch in &try_table.catches {
                match catch {
                    wasmparser::Catch::One { tag, label } => {
                        p.write_str(" (");
                        write_keyword(p, "catch");
                        p.write_str(" ");
                        print_tag_idx(ctx, p, *tag);
                        p.write_str(" ");
                        print_br_target(ctx, p, func_idx, *label, label_indices);
                        p.write_str(")");
                    }
                    wasmparser::Catch::OneRef { tag, label } => {
                        p.write_str(" (");
                        write_keyword(p, "catch_ref");
                        p.write_str(" ");
                        print_tag_idx(ctx, p, *tag);
                        p.write_str(" ");
                        print_br_target(ctx, p, func_idx, *label, label_indices);
                        p.write_str(")");
                    }
                    wasmparser::Catch::All { label } => {
                        p.write_str(" (");
                        write_keyword(p, "catch_all");
                        p.write_str(" ");
                        print_br_target(ctx, p, func_idx, *label, label_indices);
                        p.write_str(")");
                    }
                    wasmparser::Catch::AllRef { label } => {
                        p.write_str(" (");
                        write_keyword(p, "catch_all_ref");
                        p.write_str(" ");
                        print_br_target(ctx, p, func_idx, *label, label_indices);
                        p.write_str(")");
                    }
                }
            }
            if label_name.is_none() {
                p.push_style(Style::Comment);
                p.write_str(&format!(" ;; label = @{}", cur_depth));
                p.pop_style();
            }
            label_indices.push(*label_count);
            *label_count += 1;
            p.indent();
            return;
        }
        _ => {}
    }

    write_keyword(p, &instruction_wat_name(instr));

    match instr {
        // Control flow
        Instruction::Br { relative_depth } | Instruction::BrIf { relative_depth } => {
            p.write_str(" ");
            print_br_target(ctx, p, func_idx, *relative_depth, label_indices);
        }
        Instruction::BrTable { targets } => {
            for t in &targets.targets {
                p.write_str(" ");
                print_br_target(ctx, p, func_idx, *t, label_indices);
            }
            p.write_str(" ");
            print_br_target(ctx, p, func_idx, targets.default, label_indices);
        }
        Instruction::Call { function_index } | Instruction::ReturnCall { function_index } => {
            p.write_str(" ");
            print_func_idx(ctx, p, *function_index);
        }
        Instruction::CallIndirect {
            type_index,
            table_index,
            ..
        }
        | Instruction::ReturnCallIndirect {
            type_index,
            table_index,
        } => {
            if *table_index != 0 {
                p.write_str(" ");
                print_table_idx(ctx, p, *table_index);
            }
            p.write_str(" (");
            write_keyword(p, "type");
            p.write_str(" ");
            print_type_idx(ctx, p, *type_index);
            p.write_str(")");
        }
        Instruction::CallRef { type_index } | Instruction::ReturnCallRef { type_index } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *type_index);
        }

        // Parametric
        Instruction::TypedSelect { ty } => {
            p.write_str(" (");
            write_keyword(p, "result");
            p.write_str(" ");
            print_val_type_ctx(Some(ctx), p, *ty);
            p.write_str(")");
        }
        Instruction::TypedSelectMulti { tys } => {
            p.write_str(" (");
            write_keyword(p, "result");
            for ty in tys.iter() {
                p.write_str(" ");
                print_val_type(p, *ty);
            }
            p.write_str(")");
        }

        // Variable
        Instruction::LocalGet { local_index }
        | Instruction::LocalSet { local_index }
        | Instruction::LocalTee { local_index } => {
            p.write_str(" ");
            print_local_idx(ctx, p, func_idx, *local_index);
        }
        Instruction::GlobalGet { global_index } | Instruction::GlobalSet { global_index } => {
            p.write_str(" ");
            print_global_idx(ctx, p, *global_index);
        }

        // Memory: 1-byte natural alignment
        Instruction::I32Load8S { memarg }
        | Instruction::I32Load8U { memarg }
        | Instruction::I32Store8 { memarg }
        | Instruction::I64Load8S { memarg }
        | Instruction::I64Load8U { memarg }
        | Instruction::I64Store8 { memarg }
        | Instruction::V128Load8Splat { memarg } => print_memarg_natural(ctx, p, memarg, 1),

        // Memory: 2-byte natural alignment
        Instruction::I32Load16S { memarg }
        | Instruction::I32Load16U { memarg }
        | Instruction::I32Store16 { memarg }
        | Instruction::I64Load16S { memarg }
        | Instruction::I64Load16U { memarg }
        | Instruction::I64Store16 { memarg }
        | Instruction::V128Load16Splat { memarg } => print_memarg_natural(ctx, p, memarg, 2),

        // Memory: 4-byte natural alignment
        Instruction::I32Load { memarg }
        | Instruction::F32Load { memarg }
        | Instruction::I32Store { memarg }
        | Instruction::F32Store { memarg }
        | Instruction::I64Load32S { memarg }
        | Instruction::I64Load32U { memarg }
        | Instruction::I64Store32 { memarg }
        | Instruction::V128Load32Splat { memarg }
        | Instruction::V128Load32Zero { memarg } => print_memarg_natural(ctx, p, memarg, 4),

        // Memory: 8-byte natural alignment
        Instruction::I64Load { memarg }
        | Instruction::F64Load { memarg }
        | Instruction::I64Store { memarg }
        | Instruction::F64Store { memarg }
        | Instruction::V128Load8x8S { memarg }
        | Instruction::V128Load8x8U { memarg }
        | Instruction::V128Load16x4S { memarg }
        | Instruction::V128Load16x4U { memarg }
        | Instruction::V128Load32x2S { memarg }
        | Instruction::V128Load32x2U { memarg }
        | Instruction::V128Load64Splat { memarg }
        | Instruction::V128Load64Zero { memarg } => print_memarg_natural(ctx, p, memarg, 8),

        // Memory: 16-byte natural alignment
        Instruction::V128Load { memarg } | Instruction::V128Store { memarg } => {
            print_memarg_natural(ctx, p, memarg, 16)
        }

        // V128 lane memory: memarg + lane index
        Instruction::V128Load8Lane { memarg, lane }
        | Instruction::V128Store8Lane { memarg, lane } => {
            print_memarg_natural(ctx, p, memarg, 1);
            p.write_str(" ");
            p.write_str(&lane.to_string());
        }
        Instruction::V128Load16Lane { memarg, lane }
        | Instruction::V128Store16Lane { memarg, lane } => {
            print_memarg_natural(ctx, p, memarg, 2);
            p.write_str(" ");
            p.write_str(&lane.to_string());
        }
        Instruction::V128Load32Lane { memarg, lane }
        | Instruction::V128Store32Lane { memarg, lane } => {
            print_memarg_natural(ctx, p, memarg, 4);
            p.write_str(" ");
            p.write_str(&lane.to_string());
        }
        Instruction::V128Load64Lane { memarg, lane }
        | Instruction::V128Store64Lane { memarg, lane } => {
            print_memarg_natural(ctx, p, memarg, 8);
            p.write_str(" ");
            p.write_str(&lane.to_string());
        }

        Instruction::MemorySize { mem, .. } | Instruction::MemoryGrow { mem, .. } => {
            if *mem != 0 {
                p.write_str(" ");
                print_memory_idx(ctx, p, *mem);
            }
        }

        // Constants
        Instruction::I32Const { value } => {
            p.write_str(" ");
            p.push_style(Style::Literal);
            p.write_str(&value.to_string());
            p.pop_style();
        }
        Instruction::I64Const { value } => {
            p.write_str(" ");
            p.push_style(Style::Literal);
            p.write_str(&value.to_string());
            p.pop_style();
        }
        Instruction::F32Const { value } => {
            p.write_str(" ");
            print_f32(p, *value);
        }
        Instruction::F64Const { value } => {
            p.write_str(" ");
            print_f64(p, *value);
        }

        // V128
        Instruction::V128Const { value } => {
            p.write_str(" ");
            write_type_kw(p, "i32x4");
            let bytes = value.bytes();
            for chunk in bytes.chunks(4) {
                let v = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                p.write_str(" ");
                p.push_style(Style::Literal);
                p.write_str(&format!("0x{:08x}", v));
                p.pop_style();
            }
        }

        // Ref instructions
        Instruction::RefNull { hty } => {
            p.write_str(" ");
            print_heap_type_ctx(Some(ctx), p, *hty);
        }
        Instruction::RefFunc { function_index } => {
            p.write_str(" ");
            print_func_idx(ctx, p, *function_index);
        }

        // Table operations
        Instruction::TableGet { table }
        | Instruction::TableSet { table }
        | Instruction::TableGrow { table }
        | Instruction::TableSize { table }
        | Instruction::TableFill { table } => {
            p.write_str(" ");
            print_table_idx(ctx, p, *table);
        }
        Instruction::TableCopy {
            dst_table,
            src_table,
        } => {
            if *dst_table != 0 || *src_table != 0 {
                p.write_str(" ");
                print_table_idx(ctx, p, *dst_table);
                p.write_str(" ");
                print_table_idx(ctx, p, *src_table);
            }
        }
        Instruction::TableInit { elem_index, table } => {
            if *table != 0 {
                p.write_str(" ");
                print_table_idx(ctx, p, *table);
            }
            p.write_str(" ");
            print_elem_idx(ctx, p, *elem_index);
        }
        Instruction::ElemDrop { elem_index } => {
            p.write_str(" ");
            print_elem_idx(ctx, p, *elem_index);
        }

        // Memory operations
        Instruction::MemoryInit { data_index, mem } => {
            if *mem != 0 {
                p.write_str(" ");
                print_memory_idx(ctx, p, *mem);
            }
            p.write_str(" ");
            print_data_idx(ctx, p, *data_index);
        }
        Instruction::DataDrop { data_index } => {
            p.write_str(" ");
            print_data_idx(ctx, p, *data_index);
        }
        Instruction::MemoryCopy { dst_mem, src_mem } => {
            if *dst_mem != 0 || *src_mem != 0 {
                p.write_str(" ");
                print_memory_idx(ctx, p, *dst_mem);
                p.write_str(" ");
                print_memory_idx(ctx, p, *src_mem);
            }
        }
        Instruction::MemoryFill { mem } => {
            if *mem != 0 {
                p.write_str(" ");
                print_memory_idx(ctx, p, *mem);
            }
        }

        // Cast instructions
        Instruction::RefCastNonNull { hty } | Instruction::RefTestNonNull { hty } => {
            p.write_str(" ");
            if let Some(rt) = wasmparser::RefType::new(false, *hty) {
                print_ref_type_ctx(Some(ctx), p, rt);
            } else {
                p.write_str("(ref ");
                print_heap_type_ctx(Some(ctx), p, *hty);
                p.write_str(")");
            }
        }
        Instruction::RefCastNullable { hty } | Instruction::RefTestNullable { hty } => {
            p.write_str(" ");
            if let Some(rt) = wasmparser::RefType::new(true, *hty) {
                print_ref_type_ctx(Some(ctx), p, rt);
            } else {
                p.write_str("(ref null ");
                print_heap_type_ctx(Some(ctx), p, *hty);
                p.write_str(")");
            }
        }
        Instruction::BrOnCast {
            relative_depth,
            from_ref_type,
            to_ref_type,
        }
        | Instruction::BrOnCastFail {
            relative_depth,
            from_ref_type,
            to_ref_type,
        } => {
            p.write_str(" ");
            print_br_target(ctx, p, func_idx, *relative_depth, label_indices);
            p.write_str(" ");
            print_ref_type_ctx(Some(ctx), p, *from_ref_type);
            p.write_str(" ");
            print_ref_type_ctx(Some(ctx), p, *to_ref_type);
        }
        Instruction::BrOnNull { relative_depth } | Instruction::BrOnNonNull { relative_depth } => {
            p.write_str(" ");
            print_br_target(ctx, p, func_idx, *relative_depth, label_indices);
        }

        // Throw/tag instructions
        Instruction::Throw { tag_index } => {
            p.write_str(" ");
            print_tag_idx(ctx, p, *tag_index);
        }

        // SIMD shuffle
        Instruction::I8x16Shuffle { lanes } => {
            for lane in lanes.iter() {
                p.write_str(" ");
                p.write_str(&lane.to_string());
            }
        }

        // SIMD lane operations
        Instruction::I8x16ExtractLaneS { lane }
        | Instruction::I8x16ExtractLaneU { lane }
        | Instruction::I8x16ReplaceLane { lane }
        | Instruction::I16x8ExtractLaneS { lane }
        | Instruction::I16x8ExtractLaneU { lane }
        | Instruction::I16x8ReplaceLane { lane }
        | Instruction::I32x4ExtractLane { lane }
        | Instruction::I32x4ReplaceLane { lane }
        | Instruction::I64x2ExtractLane { lane }
        | Instruction::I64x2ReplaceLane { lane }
        | Instruction::F32x4ExtractLane { lane }
        | Instruction::F32x4ReplaceLane { lane }
        | Instruction::F64x2ExtractLane { lane }
        | Instruction::F64x2ReplaceLane { lane } => {
            p.write_str(" ");
            p.write_str(&lane.to_string());
        }

        // Atomic memory instructions
        Instruction::MemoryAtomicNotify { memarg }
        | Instruction::MemoryAtomicWait32 { memarg }
        | Instruction::MemoryAtomicWait64 { memarg }
        | Instruction::I32AtomicLoad { memarg }
        | Instruction::I64AtomicLoad { memarg }
        | Instruction::I32AtomicLoad8U { memarg }
        | Instruction::I32AtomicLoad16U { memarg }
        | Instruction::I64AtomicLoad8U { memarg }
        | Instruction::I64AtomicLoad16U { memarg }
        | Instruction::I64AtomicLoad32U { memarg }
        | Instruction::I32AtomicStore { memarg }
        | Instruction::I64AtomicStore { memarg }
        | Instruction::I32AtomicStore8 { memarg }
        | Instruction::I32AtomicStore16 { memarg }
        | Instruction::I64AtomicStore8 { memarg }
        | Instruction::I64AtomicStore16 { memarg }
        | Instruction::I64AtomicStore32 { memarg }
        | Instruction::I32AtomicRmwAdd { memarg }
        | Instruction::I64AtomicRmwAdd { memarg }
        | Instruction::I32AtomicRmw8AddU { memarg }
        | Instruction::I32AtomicRmw16AddU { memarg }
        | Instruction::I64AtomicRmw8AddU { memarg }
        | Instruction::I64AtomicRmw16AddU { memarg }
        | Instruction::I64AtomicRmw32AddU { memarg }
        | Instruction::I32AtomicRmwSub { memarg }
        | Instruction::I64AtomicRmwSub { memarg }
        | Instruction::I32AtomicRmw8SubU { memarg }
        | Instruction::I32AtomicRmw16SubU { memarg }
        | Instruction::I64AtomicRmw8SubU { memarg }
        | Instruction::I64AtomicRmw16SubU { memarg }
        | Instruction::I64AtomicRmw32SubU { memarg }
        | Instruction::I32AtomicRmwAnd { memarg }
        | Instruction::I64AtomicRmwAnd { memarg }
        | Instruction::I32AtomicRmw8AndU { memarg }
        | Instruction::I32AtomicRmw16AndU { memarg }
        | Instruction::I64AtomicRmw8AndU { memarg }
        | Instruction::I64AtomicRmw16AndU { memarg }
        | Instruction::I64AtomicRmw32AndU { memarg }
        | Instruction::I32AtomicRmwOr { memarg }
        | Instruction::I64AtomicRmwOr { memarg }
        | Instruction::I32AtomicRmw8OrU { memarg }
        | Instruction::I32AtomicRmw16OrU { memarg }
        | Instruction::I64AtomicRmw8OrU { memarg }
        | Instruction::I64AtomicRmw16OrU { memarg }
        | Instruction::I64AtomicRmw32OrU { memarg }
        | Instruction::I32AtomicRmwXor { memarg }
        | Instruction::I64AtomicRmwXor { memarg }
        | Instruction::I32AtomicRmw8XorU { memarg }
        | Instruction::I32AtomicRmw16XorU { memarg }
        | Instruction::I64AtomicRmw8XorU { memarg }
        | Instruction::I64AtomicRmw16XorU { memarg }
        | Instruction::I64AtomicRmw32XorU { memarg }
        | Instruction::I32AtomicRmwXchg { memarg }
        | Instruction::I64AtomicRmwXchg { memarg }
        | Instruction::I32AtomicRmw8XchgU { memarg }
        | Instruction::I32AtomicRmw16XchgU { memarg }
        | Instruction::I64AtomicRmw8XchgU { memarg }
        | Instruction::I64AtomicRmw16XchgU { memarg }
        | Instruction::I64AtomicRmw32XchgU { memarg }
        | Instruction::I32AtomicRmwCmpxchg { memarg }
        | Instruction::I64AtomicRmwCmpxchg { memarg }
        | Instruction::I32AtomicRmw8CmpxchgU { memarg }
        | Instruction::I32AtomicRmw16CmpxchgU { memarg }
        | Instruction::I64AtomicRmw8CmpxchgU { memarg }
        | Instruction::I64AtomicRmw16CmpxchgU { memarg }
        | Instruction::I64AtomicRmw32CmpxchgU { memarg } => {
            print_memarg(ctx, p, memarg);
        }

        // GC instructions
        Instruction::StructNew { struct_type_index }
        | Instruction::StructNewDefault { struct_type_index } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *struct_type_index);
        }
        Instruction::StructGet {
            struct_type_index,
            field_index,
        }
        | Instruction::StructGetS {
            struct_type_index,
            field_index,
        }
        | Instruction::StructGetU {
            struct_type_index,
            field_index,
        }
        | Instruction::StructSet {
            struct_type_index,
            field_index,
        } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *struct_type_index);
            p.write_str(" ");
            print_idx(
                p,
                ctx.field_name(*struct_type_index, *field_index),
                *field_index,
            );
        }
        Instruction::ArrayNew { array_type_index }
        | Instruction::ArrayNewDefault { array_type_index }
        | Instruction::ArrayGet { array_type_index }
        | Instruction::ArrayGetS { array_type_index }
        | Instruction::ArrayGetU { array_type_index }
        | Instruction::ArraySet { array_type_index }
        | Instruction::ArrayFill { array_type_index } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *array_type_index);
        }
        Instruction::ArrayNewFixed {
            array_type_index,
            array_size,
        } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *array_type_index);
            p.write_str(" ");
            p.write_str(&array_size.to_string());
        }
        Instruction::ArrayNewData {
            array_type_index,
            array_data_index,
        }
        | Instruction::ArrayInitData {
            array_type_index,
            array_data_index,
        } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *array_type_index);
            p.write_str(" ");
            print_data_idx(ctx, p, *array_data_index);
        }
        Instruction::ArrayNewElem {
            array_type_index,
            array_elem_index,
        }
        | Instruction::ArrayInitElem {
            array_type_index,
            array_elem_index,
        } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *array_type_index);
            p.write_str(" ");
            print_elem_idx(ctx, p, *array_elem_index);
        }
        Instruction::ArrayCopy {
            array_type_index_dst,
            array_type_index_src,
        } => {
            p.write_str(" ");
            print_type_idx(ctx, p, *array_type_index_dst);
            p.write_str(" ");
            print_type_idx(ctx, p, *array_type_index_src);
        }

        // Everything else: simple ops with no args
        _ => {}
    }

}

fn print_block_label_and_type(
    ctx: &PrintContext,
    p: &mut dyn Printer,
    func_idx: u32,
    bt: wasmparser::BlockType,
    cur_depth: u32,
    label_count: u32,
) {
    let label_names = ctx.module.names.label_names.get(&func_idx);
    let label_name = label_names.and_then(|m| m.get(&label_count));
    if let Some(name) = label_name {
        p.write_str(" ");
        write_name(p, name);
    }
    print_blocktype(ctx, p, bt);
    if label_name.is_none() {
        p.push_style(Style::Comment);
        p.write_str(&format!(" ;; label = @{}", cur_depth));
        p.pop_style();
    }
}

fn print_blocktype(ctx: &PrintContext, p: &mut dyn Printer, bt: wasmparser::BlockType) {
    match bt {
        wasmparser::BlockType::Empty => {}
        wasmparser::BlockType::Type(t) => {
            p.write_str(" (");
            write_keyword(p, "result");
            p.write_str(" ");
            print_val_type_ctx(Some(ctx), p, t);
            p.write_str(")");
        }
        wasmparser::BlockType::FuncType(idx) => {
            p.write_str(" (");
            write_keyword(p, "type");
            p.write_str(" ");
            print_type_idx(ctx, p, idx);
            p.write_str(")");
            // Also expand params/results like wasmprinter does
            if let Some(ft) = get_func_type(ctx, idx) {
                if !ft.params.is_empty() {
                    p.write_str(" (");
                    write_keyword(p, "param");
                    for param in &ft.params {
                        p.write_str(" ");
                        print_val_type_ctx(Some(ctx), p, *param);
                    }
                    p.write_str(")");
                }
                if !ft.results.is_empty() {
                    p.write_str(" (");
                    write_keyword(p, "result");
                    for result in &ft.results {
                        p.write_str(" ");
                        print_val_type_ctx(Some(ctx), p, *result);
                    }
                    p.write_str(")");
                }
            }
        }
    }
}

fn print_local_idx(ctx: &PrintContext, p: &mut dyn Printer, func_idx: u32, local_idx: u32) {
    let name = ctx
        .module
        .names
        .local_names
        .get(&func_idx)
        .and_then(|m| m.get(&local_idx))
        .map(|s| s.as_str());
    p.begin_xref(ItemId::Local { func: func_idx, local: local_idx });
    print_idx(p, name, local_idx);
    p.end_xref();
}

fn print_br_target(
    ctx: &PrintContext,
    p: &mut dyn Printer,
    func_idx: u32,
    relative_depth: u32,
    label_indices: &[u32],
) {
    // label_indices[0] = function body (sentinel), label_indices[1] = first block, etc.
    // wasmprinter's cur_depth starts at 0 for function body, so we use len-1
    let cur_depth = (label_indices.len() as u32).saturating_sub(1);
    let label_names = ctx.module.names.label_names.get(&func_idx);

    // i = wasmprinter-style absolute depth of the target (0 = function, 1 = first block, etc.)
    let i = cur_depth.checked_sub(relative_depth);

    // Look up sequential label index from the stack
    // label_indices[0] = function body (no label name), label_indices[1] = first block label
    // wasmprinter's label_indices stack doesn't include function body, so index mapping:
    // wasmprinter i=0 means function (no label), i=1 means label_indices[0] in wasmprinter
    // Our label_indices[i] where i is the wasmprinter depth (our stack includes function at [0])
    let label_idx = i.and_then(|idx| label_indices.get(idx as usize).copied());

    let name = label_idx
        .filter(|&li| li != u32::MAX)
        .and_then(|li| label_names.and_then(|m| m.get(&li)));

    // Check for name conflicts (shadowing): if a shallower (more recent) label has the same name,
    // we can't use the name because br resolves to the nearest matching name
    let name_conflict = if let Some(target_name) = name {
        if let Some(idx) = i {
            // Check labels from idx+1 to end (shallower = more recently pushed)
            label_indices[(idx as usize + 1)..].iter().any(|other_li| {
                if *other_li == u32::MAX {
                    return false;
                }
                label_names
                    .and_then(|m| m.get(other_li))
                    .map(|n| n == target_name)
                    .unwrap_or(false)
            })
        } else {
            false
        }
    } else {
        false
    };

    match name {
        Some(name) if !name_conflict => {
            p.write_str("$");
            print_id_name(p, name);
        }
        _ => {
            p.push_style(Style::Literal);
            p.write_str(&relative_depth.to_string());
            p.pop_style();
            // Add (;@N;) annotation for unnamed labels, but not for the function itself (i=0)
            if let Some(idx) = i {
                if idx > 0 {
                    p.push_style(Style::Comment);
                    p.write_str(&format!(" (;@{};)", idx));
                    p.pop_style();
                }
            }
        }
    }
}

fn print_memarg_natural(
    ctx: &PrintContext,
    p: &mut dyn Printer,
    memarg: &wasmparser::MemArg,
    natural_align: u32,
) {
    let align = 1u64 << memarg.align;
    if memarg.memory != 0 {
        p.write_str(" ");
        print_memory_idx(ctx, p, memarg.memory);
    }
    if memarg.offset != 0 {
        p.write_str(" ");
        write_keyword(p, "offset=");
        p.push_style(Style::Literal);
        p.write_str(&memarg.offset.to_string());
        p.pop_style();
    }
    if align != natural_align as u64 {
        p.write_str(" ");
        write_keyword(p, "align=");
        p.push_style(Style::Literal);
        p.write_str(&align.to_string());
        p.pop_style();
    }
}

/// Print memarg using the built-in max_align as natural alignment
fn print_memarg(ctx: &PrintContext, p: &mut dyn Printer, memarg: &wasmparser::MemArg) {
    if memarg.memory != 0 {
        p.write_str(" ");
        print_memory_idx(ctx, p, memarg.memory);
    }
    if memarg.offset != 0 {
        p.write_str(" ");
        write_keyword(p, "offset=");
        p.push_style(Style::Literal);
        p.write_str(&memarg.offset.to_string());
        p.pop_style();
    }
    if memarg.align != memarg.max_align {
        let align = 1u64 << memarg.align;
        p.write_str(" ");
        write_keyword(p, "align=");
        p.push_style(Style::Literal);
        p.write_str(&align.to_string());
        p.pop_style();
    }
}

macro_rules! print_float {
    ($print_fn:ident, $float:ty, $uint:ty, $sint:ty, $exp_width:expr) => {
        fn $print_fn(p: &mut dyn Printer, bits: $uint) {
            let f = <$float>::from_bits(bits);
            let int_width = std::mem::size_of::<$uint>() * 8;
            let mantissa_width = int_width - 1 - $exp_width;
            let sign_bit = 1 as $uint << (int_width - 1);
            let mantissa_mask = (1 as $uint << mantissa_width) - 1;
            let bias = (1 << ($exp_width - 1)) - 1i32;
            let min_exp = -(1i32 << ($exp_width - 1)) + 1;

            p.push_style(Style::Literal);

            // Handle sign
            if bits & sign_bit != 0 {
                p.write_str("-");
            }
            let bits_abs = bits & !sign_bit;

            if f.is_nan() {
                let payload = bits_abs & mantissa_mask;
                let canonical = 1 as $uint << (mantissa_width - 1);
                if payload == canonical {
                    p.write_str("nan");
                } else {
                    p.write_str(&format!("nan:0x{:x}", payload));
                }
                p.pop_style();
                p.push_style(Style::Comment);
                p.write_str(" (;=NaN;)");
                p.pop_style();
                return;
            }

            if f.is_infinite() {
                p.write_str("inf");
                p.pop_style();
                p.push_style(Style::Comment);
                p.write_str(&format!(" (;={};)", f));
                p.pop_style();
                return;
            }

            // Extract exponent and mantissa
            p.write_str("0x");

            if bits_abs == 0 {
                p.write_str("0p+0");
            } else {
                // Match wasmprinter's exponent extraction exactly
                let mut exponent = (((bits << 1) as $sint) >> (mantissa_width as $sint + 1)).wrapping_sub(bias as $sint);
                exponent = (exponent << (int_width as $sint - $exp_width as $sint)) >> (int_width as $sint - $exp_width as $sint);
                let mut fraction = bits_abs & mantissa_mask;

                p.write_str("1");
                if fraction > 0 {
                    fraction <<= int_width - mantissa_width;

                    // Subnormal: normalize
                    if exponent == min_exp as $sint {
                        let leading = fraction.leading_zeros();
                        if (leading as usize) < int_width - 1 {
                            fraction <<= leading + 1;
                        } else {
                            fraction = 0;
                        }
                        exponent -= leading as $sint;
                    }

                    p.write_str(".");
                    while fraction > 0 {
                        let digit = (fraction >> (int_width - 4)) as u8;
                        p.write_str(&format!("{:x}", digit));
                        fraction <<= 4;
                    }
                }
                p.write_str(&format!("p{:+}", exponent));
            }

            // Decimal comment
            p.pop_style();
            p.push_style(Style::Comment);
            p.write_str(&format!(" (;={};)", f));
            p.pop_style();
        }
    };
}

print_float!(print_f32_inner, f32, u32, i32, 8);
print_float!(print_f64_inner, f64, u64, i64, 11);

fn print_f32(p: &mut dyn Printer, val: wasmparser::Ieee32) {
    print_f32_inner(p, val.bits());
}

fn print_f64(p: &mut dyn Printer, val: wasmparser::Ieee64) {
    print_f64_inner(p, val.bits());
}

// Fallback: generate the instruction name from the visit function name via macro
macro_rules! define_instruction_name {
    ($( @$proposal:ident $op:ident $({ $($arg:ident : $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        fn instruction_name(instr: &Instruction) -> &'static str {
            match instr {
                $(
                    Instruction::$op { .. } => stringify!($visit),
                )*
            }
        }
    };
}

wasmparser::for_each_operator!(define_instruction_name);

/// Convert a visit function name like "visit_i32_add" to "i32.add"
fn visit_name_to_wat(visit: &str) -> String {
    let name = &visit[6..]; // strip "visit_"
                            // Replace underscores with dots for instruction categories
                            // The pattern: first segment before _ that is a type prefix (i32, i64, f32, f64, v128, etc.)
                            // gets a dot separator; other underscores stay as underscores
    let prefixes = [
        "i32_", "i64_", "f32_", "f64_", "v128_", "i8x16_", "i16x8_", "i32x4_", "i64x2_", "f32x4_",
        "f64x2_", "memory_", "table_", "ref_", "struct_", "array_", "any_", "global_", "local_",
        "i31_", "extern_", "data_", "elem_", "atomic_",
    ];

    for prefix in &prefixes {
        if name.starts_with(prefix) {
            let (head, tail) = name.split_at(prefix.len() - 1);
            let rest = &tail[1..];
            // Handle second-level dot for "atomic." sub-prefix
            if let Some(inner_rest) = rest.strip_prefix("atomic_") {
                // Handle third-level dot for "rmw." sub-prefix
                // Patterns: rmw_add, rmw8_add_u, rmw16_add_u, rmw32_add_u
                for rmw_prefix in &["rmw_", "rmw8_", "rmw16_", "rmw32_"] {
                    if let Some(op) = inner_rest.strip_prefix(rmw_prefix) {
                        let rmw_name = &rmw_prefix[..rmw_prefix.len() - 1]; // "rmw", "rmw8", etc.
                        return format!("{}.atomic.{}.{}", head, rmw_name, op);
                    }
                }
                return format!("{}.atomic.{}", head, inner_rest);
            }
            return format!("{}.{}", head, rest);
        }
    }

    name.to_string()
}

fn instruction_wat_name(instr: &Instruction) -> std::borrow::Cow<'static, str> {
    match instr {
        Instruction::RefTestNonNull { .. } | Instruction::RefTestNullable { .. } => {
            "ref.test".into()
        }
        Instruction::RefCastNonNull { .. } | Instruction::RefCastNullable { .. } => {
            "ref.cast".into()
        }
        Instruction::RefCastDescEqNonNull { .. } | Instruction::RefCastDescEqNullable { .. } => {
            "ref.cast_desc_eq".into()
        }
        Instruction::TypedSelect { .. } | Instruction::TypedSelectMulti { .. } => "select".into(),
        _ => visit_name_to_wat(instruction_name(instr)).into(),
    }
}

