#[allow(warnings)]
mod bindings;

use bindings::exports::local::module::module::{Guest, GuestModule, Range, Item, PrintPart};

struct Component;

struct PlainWriter {
    result: String,
    range: Range,
    current: usize,
}

impl wasmprinter::Print for PlainWriter {
    fn write_str(&mut self, s: &str) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.result.push_str(s);
        Ok(())
    }

    fn start_line(&mut self, binary_offset: Option<usize>) {
        if let Some(binary_offset) = binary_offset {
            self.current = binary_offset;
        }
    }
}

struct RichWriter {
    parts: Vec<PrintPart>,
    range: Range,
    current: usize,
}

impl wasmprinter::Print for RichWriter {
    fn write_str(&mut self, s: &str) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Str(s.to_string()));
        Ok(())
    }

    fn start_line(&mut self, binary_offset: Option<usize>) {
        if let Some(binary_offset) = binary_offset {
            self.current = binary_offset;
        }
    }

    fn start_name(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Name);
        Ok(())
    }

    fn start_literal(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Literal);
        Ok(())
    }

    fn start_keyword(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Keyword);
        Ok(())
    }

    fn start_type(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Type);
        Ok(())
    }

    fn start_comment(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Comment);
        Ok(())
    }

    fn reset_color(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::Reset);
        Ok(())
    }
}

impl Guest for Component {
    type Module = Module;
}

struct Module {
    bytes: Vec<u8>,
}

impl GuestModule for Module {
    fn new(init: Vec<u8>) -> Self {
        Module {
            bytes: init,
        }
    }

    fn print_rich(&self, r: Range) -> Result<Vec<PrintPart>, String> {
        let config = wasmprinter::Config::new();
        let mut writer = RichWriter {
            parts: Vec::new(),
            range: r,
            current: 0
        };
        let result = config.print(&self.bytes, &mut writer);
        result.map(|_| writer.parts).map_err(|e| e.to_string())
    }

    fn print_plain(&self, r: Range) -> Result<String, String> {
        let config = wasmprinter::Config::new();
        let mut writer = PlainWriter {
            result: String::new(),
            range: r,
            current: 0
        };
        let result = config.print(&self.bytes, &mut writer);
        result.map(|_| writer.result).map_err(|e| e.to_string())
    }

    fn items(&self) -> Vec<Item> {
        let items = gather_items(&self.bytes);
        items.unwrap_or(Vec::new())
    }
}

fn convert_range(r: std::ops::Range<usize>) -> Range {
    Range {
        start: r.start as u32,
        end: r.end as u32,
    }
}

fn gather_items(mut bytes: &[u8]) -> anyhow::Result<Vec<Item>> {
    use wasmparser::*;
    let mut parser = Parser::new(0);
    let mut items = Vec::new();

    let mut func_index = 0;
    loop {
        let payload = match parser.parse(bytes, true)? {
            Chunk::NeedMoreData(_) => unreachable!(),
            Chunk::Parsed { payload, consumed } => {
                bytes = &bytes[consumed..];
                payload
            }
        };
        match payload {
            Payload::TypeSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("types"),
                });
            }
            Payload::ImportSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("imports"),
                });
                // TODO: add imported functions to func_index
            }
            Payload::FunctionSection(reader) => {}
            Payload::TableSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("tables"),
                });
            }
            Payload::MemorySection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("memories"),
                });
            }
            Payload::TagSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("tags"),
                });
            }
            Payload::GlobalSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("globals"),
                });
            }
            Payload::ExportSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("exports"),
                });
            }
            Payload::StartSection { func, range } => {
                items.push(Item {
                    range: convert_range(range),
                    name: format!("start"),
                });
            }
            Payload::ElementSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("elements"),
                });
            }
            Payload::CodeSectionStart { range, .. } => {
                items.push(Item {
                    range: convert_range(range),
                    name: format!("functions"),
                });
            }
            Payload::CodeSectionEntry(body) => {
                func_index += 1;
                items.push(Item {
                    range: convert_range(body.range()),
                    name: format!("func {func_index}"),
                });
            }
            Payload::DataCountSection { .. } => {
            }
            Payload::DataSection(s) => {
                items.push(Item {
                    range: convert_range(s.range()),
                    name: format!("data"),
                });
            }

            Payload::End(_) => {
                break;
            }

            _ => {},
        }
    }

    Ok(items)
}

bindings::export!(Component with_types_in bindings);
