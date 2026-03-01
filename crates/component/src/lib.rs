#[allow(warnings)]
mod bindings;

use bindings::exports::local::module::module::{
    DefinitionId, Guest, GuestModule, Item, LocalId, PrintPart, Range, ValidateError,
};
use waside::{ItemId, Module as WasmModule, PrintContext, Printer, Style};

struct Component;

struct PlainWriter {
    result: String,
    indent_level: usize,
    at_line_start: bool,
}

impl PlainWriter {
    fn write_indent(&mut self) {
        if self.at_line_start {
            for _ in 0..self.indent_level.min(50) {
                self.result.push_str("  ");
            }
            self.at_line_start = false;
        }
    }
}

impl Printer for PlainWriter {
    fn write_str(&mut self, s: &str) {
        self.write_indent();
        self.result.push_str(s);
    }

    fn newline(&mut self, _offset: Option<usize>) {
        self.result.push('\n');
        self.at_line_start = true;
    }

    fn push_style(&mut self, _: Style) {}
    fn pop_style(&mut self) {}
    fn begin_xref(&mut self, _: ItemId) {}
    fn end_xref(&mut self) {}

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }
}

struct RichWriter {
    parts: Vec<PrintPart>,
    indent_level: usize,
    at_line_start: bool,
}

impl RichWriter {
    fn write_indent(&mut self) {
        if self.at_line_start {
            let spaces = "  ".repeat(self.indent_level.min(50));
            if !spaces.is_empty() {
                if let Some(PrintPart::Str(last_str)) = self.parts.last_mut() {
                    last_str.push_str(&spaces);
                } else {
                    self.parts.push(PrintPart::Str(spaces));
                }
            }
            self.at_line_start = false;
        }
    }
}

impl Printer for RichWriter {
    fn write_str(&mut self, s: &str) {
        self.write_indent();
        if let Some(PrintPart::Str(last_str)) = self.parts.last_mut() {
            last_str.push_str(s);
            return;
        }
        self.parts.push(PrintPart::Str(s.to_string()));
    }

    fn newline(&mut self, offset: Option<usize>) {
        self.parts.push(PrintPart::NewLine(offset.unwrap_or(0) as u32));
        self.at_line_start = true;
    }

    fn push_style(&mut self, style: Style) {
        self.write_indent();
        let part = match style {
            Style::Name => PrintPart::Name,
            Style::Literal => PrintPart::Literal,
            Style::Keyword => PrintPart::Keyword,
            Style::Type => PrintPart::Type,
            Style::Comment => PrintPart::Comment,
            Style::Punctuation | Style::Default => return,
        };
        self.parts.push(part);
    }

    fn pop_style(&mut self) {
        self.parts.push(PrintPart::Reset);
    }

    fn begin_xref(&mut self, id: ItemId) {
        self.write_indent();
        self.parts.push(PrintPart::Xref(to_definition_id(&id)));
    }

    fn end_xref(&mut self) {
        self.parts.push(PrintPart::Reset);
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }
}

impl Guest for Component {
    type Module = Module;
}

struct Module {
    bytes: Vec<u8>,
    decoded: Result<WasmModule, waside::Error>,
}

impl GuestModule for Module {
    fn new(init: Vec<u8>) -> Self {
        let bytes = if let Ok(std::borrow::Cow::Owned(b)) = wat::parse_bytes(&init) {
            b
        } else {
            init
        };
        let decoded = WasmModule::decode(&bytes);
        Module { bytes, decoded }
    }

    fn validate(&self) -> Option<ValidateError> {
        match &self.decoded {
            Ok(_) => None,
            Err(waside::Error::BinaryReader(e)) => Some(ValidateError {
                message: e.message().to_owned(),
                offset: e.offset() as u32,
            }),
            Err(e) => Some(ValidateError {
                message: e.to_string(),
                offset: 0,
            }),
        }
    }

    fn print_rich(&self, id: Option<DefinitionId>) -> Result<Vec<PrintPart>, String> {
        let module = self.decoded.as_ref().map_err(|e| e.to_string())?;
        let mut writer = RichWriter {
            parts: Vec::new(),
            indent_level: 0,
            at_line_start: true,
        };
        print_definition(module, id.as_ref(), &mut writer);
        Ok(writer.parts)
    }

    fn print_plain(&self, id: Option<DefinitionId>) -> Result<String, String> {
        let module = self.decoded.as_ref().map_err(|e| e.to_string())?;
        let mut writer = PlainWriter {
            result: String::new(),
            indent_level: 0,
            at_line_start: true,
        };
        print_definition(module, id.as_ref(), &mut writer);
        Ok(writer.result)
    }

    fn source(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn items(&self) -> Vec<Item> {
        match &self.decoded {
            Ok(module) => gather_items(&self.bytes, module),
            Err(_) => vec![Item {
                range: Range {
                    start: 0,
                    end: self.bytes.len() as u32,
                },
                raw_name: "module".to_string(),
                display_name: "module".to_string(),
                definition_id: None,
            }],
        }
    }
}

fn to_item_id(id: &DefinitionId) -> (waside::ItemId, u32) {
    match id {
        DefinitionId::Type(i) => (waside::ItemId::Type(*i), *i),
        DefinitionId::Func(i) => (waside::ItemId::Func(*i), *i),
        DefinitionId::Table(i) => (waside::ItemId::Table(*i), *i),
        DefinitionId::Memory(i) => (waside::ItemId::Memory(*i), *i),
        DefinitionId::Global(i) => (waside::ItemId::Global(*i), *i),
        DefinitionId::Element(i) => (waside::ItemId::Element(*i), *i),
        DefinitionId::Data(i) => (waside::ItemId::Data(*i), *i),
        DefinitionId::Tag(i) => (waside::ItemId::Tag(*i), *i),
        DefinitionId::Local(l) => (waside::ItemId::Local { func: l.func, local: l.local }, l.func),
    }
}

fn print_definition(module: &WasmModule, id: Option<&DefinitionId>, p: &mut dyn Printer) {
    let Some(def_id) = id else {
        module.print_to(p);
        return;
    };
    let (item_id, idx) = to_item_id(def_id);
    if let Some(item) = module.find_closest_item(&item_id) {
        let ctx = PrintContext::new(module);
        item.print(&ctx, p, idx);
    }
}

fn to_definition_id(id: &waside::ItemId) -> DefinitionId {
    match id {
        waside::ItemId::Type(i) => DefinitionId::Type(*i),
        waside::ItemId::Func(i) => DefinitionId::Func(*i),
        waside::ItemId::Table(i) => DefinitionId::Table(*i),
        waside::ItemId::Memory(i) => DefinitionId::Memory(*i),
        waside::ItemId::Global(i) => DefinitionId::Global(*i),
        waside::ItemId::Element(i) => DefinitionId::Element(*i),
        waside::ItemId::Data(i) => DefinitionId::Data(*i),
        waside::ItemId::Local { func, local } => {
            DefinitionId::Local(LocalId { func: *func, local: *local })
        }
        waside::ItemId::Tag(i) => DefinitionId::Tag(*i),
    }
}

fn span_to_range(span: waside::Span) -> Range {
    Range {
        start: span.offset as u32,
        end: (span.offset + span.len) as u32,
    }
}

fn display_name(names_map: Option<&String>, raw_name: &str) -> String {
    names_map.cloned().unwrap_or_else(|| raw_name.to_string())
}

fn gather_items(bytes: &[u8], module: &WasmModule) -> Vec<Item> {
    let mut items = Vec::new();
    let names = &module.names;

    items.push(Item {
        range: Range {
            start: 0,
            end: bytes.len() as u32,
        },
        raw_name: "module".to_string(),
        display_name: names
            .module_name
            .clone()
            .unwrap_or_else(|| "module".to_string()),
        definition_id: None,
    });

    for (id, span) in module.definitions() {
        let range = span_to_range(span);
        let (raw_name, display_name) = match &id {
            ItemId::Type(i) => {
                let raw = format!("type {i}");
                let disp = display_name(names.type_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Func(i) => {
                let raw = format!("func {i}");
                let disp = display_name(names.function_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Table(i) => {
                let raw = format!("table {i}");
                let disp = display_name(names.table_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Memory(i) => {
                let raw = format!("memory {i}");
                let disp = display_name(names.memory_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Global(i) => {
                let raw = format!("global {i}");
                let disp = display_name(names.global_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Tag(i) => {
                let raw = format!("tag {i}");
                let disp = display_name(names.tag_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Element(i) => {
                let raw = format!("elem {i}");
                let disp = display_name(names.element_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Data(i) => {
                let raw = format!("data {i}");
                let disp = display_name(names.data_names.get(i), &raw);
                (raw, disp)
            }
            ItemId::Local { .. } => continue,
        };
        items.push(Item {
            range,
            raw_name,
            display_name,
            definition_id: Some(to_definition_id(&id)),
        });
    }

    items
}

bindings::export!(Component with_types_in bindings);
