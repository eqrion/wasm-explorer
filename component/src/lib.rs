#[allow(warnings)]
mod bindings;

use anyhow::bail;
use bindings::exports::local::module::module::{
    Guest, GuestModule, Item, PrintPart, Range, ValidateError,
};
use std::collections::HashMap;

struct Component;

struct PlainWriter {
    result: String,
    range: Range,
    current: usize,
}

impl wasmprinter::Print for PlainWriter {
    fn newline(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.result.push_str("\n");
        Ok(())
    }

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
    fn newline(&mut self) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        self.parts.push(PrintPart::NewLine(self.current as u32));
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> std::io::Result<()> {
        if self.current < self.range.start as usize || self.current >= self.range.end as usize {
            return Ok(());
        }
        if let Some(PrintPart::Str(last_str)) = self.parts.last_mut() {
            last_str.push_str(s);
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
        if let Ok(std::borrow::Cow::Owned(bytes)) = wat::parse_bytes(&init) {
            Module { bytes }
        } else {
            Module { bytes: init }
        }
    }

    fn validate(&self) -> Option<ValidateError> {
        match wasmparser::validate(&self.bytes) {
            Ok(_) => None,
            Err(e) => Some(ValidateError {
                message: e.message().to_owned(),
                offset: e.offset() as u32,
            }),
        }
    }

    fn print_rich(&self, r: Range) -> Result<Vec<PrintPart>, String> {
        let config = wasmprinter::Config::new();
        let mut writer = RichWriter {
            parts: Vec::new(),
            range: r,
            current: 0,
        };
        let result = config.print(&self.bytes, &mut writer);
        result.map(|_| writer.parts).map_err(|e| e.to_string())
    }

    fn print_plain(&self, r: Range) -> Result<String, String> {
        let config = wasmprinter::Config::new();
        let mut writer = PlainWriter {
            result: String::new(),
            range: r,
            current: 0,
        };
        let result = config.print(&self.bytes, &mut writer);
        result.map(|_| writer.result).map_err(|e| e.to_string())
    }

    fn items(&self) -> Vec<Item> {
        let items = gather_items(&self.bytes);
        items.unwrap_or(Vec::new())
    }
}

fn convert_range(r: &std::ops::Range<usize>) -> Range {
    Range {
        start: r.start as u32,
        end: r.end as u32,
    }
}

struct Alias {
    name: String,
    item_name: String,
}

fn gather_aliases(mut bytes: &[u8]) -> anyhow::Result<Vec<Alias>> {
    use wasmparser::*;

    let mut aliases = Vec::new();
    let mut parser = Parser::new(0);

    'outer: loop {
        let payload = match parser.parse(bytes, true)? {
            Chunk::NeedMoreData(_) => unreachable!(),
            Chunk::Parsed { payload, consumed } => {
                bytes = &bytes[consumed..];
                payload
            }
        };

        match payload {
            Payload::CodeSectionStart { size, .. } => {
                if size as usize > bytes.len() {
                    bail!("invalid code section size");
                }
                bytes = &bytes[size as usize..];
                parser.skip_section();
            }
            Payload::CustomSection(reader) if reader.name() == "name" => {
                let binary_reader = BinaryReader::new(reader.data(), reader.data_offset());
                let reader = NameSectionReader::new(binary_reader);

                for subsection in reader {
                    let subsection = subsection?;

                    match subsection {
                        Name::Module {
                            name,
                            name_range: _,
                        } => {
                            aliases.push(Alias {
                                name: name.to_owned(),
                                item_name: "module".to_owned(),
                            });
                        }
                        Name::Function(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("func {}", alias_name.index),
                                });
                            }
                        }
                        Name::Type(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("type {}", alias_name.index),
                                });
                            }
                        }
                        Name::Tag(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("tag {}", alias_name.index),
                                });
                            }
                        }
                        Name::Table(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("table {}", alias_name.index),
                                });
                            }
                        }
                        Name::Memory(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("memory {}", alias_name.index),
                                });
                            }
                        }
                        Name::Global(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("global {}", alias_name.index),
                                });
                            }
                        }
                        Name::Element(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("element {}", alias_name.index),
                                });
                            }
                        }
                        Name::Data(section_limited) => {
                            for alias_name in section_limited {
                                let alias_name = alias_name?;
                                aliases.push(Alias {
                                    name: alias_name.name.to_owned(),
                                    item_name: format!("data {}", alias_name.index),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            Payload::End(_) => break 'outer,
            _ => {}
        }
    }

    Ok(aliases)
}

fn gather_items(mut bytes: &[u8]) -> anyhow::Result<Vec<Item>> {
    use wasmparser::*;

    let mut items = Vec::new();
    let original_bytes = bytes;

    items.push(Item {
        range: Range {
            start: 0,
            end: bytes.len() as u32,
        },
        raw_name: format!("module"),
        display_name: String::new(),
    });

    let mut parser = Parser::new(0);

    let mut func_index = 0;
    let mut global_index = 0;
    let mut memory_index = 0;
    let mut table_index = 0;
    let mut type_index = 0;
    let mut tag_index = 0;
    let mut elem_index = 0;
    let mut data_index = 0;

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
                let range = s.range();

                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("types"),
                    display_name: String::new(),
                });

                for rec_group in s {
                    let rec_group = rec_group?;
                    for (offset, _ty) in rec_group.into_types_and_offsets() {
                        if type_index != 0 {
                            items.last_mut().unwrap().range.end = offset as u32;
                        }

                        items.push(Item {
                            raw_name: format!("type {type_index}"),
                            display_name: String::new(),
                            range: Range {
                                start: offset as u32,
                                end: offset as u32,
                            },
                        });

                        type_index += 1;
                    }
                }

                if type_index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::ImportSection(s) => {
                let range = s.range();

                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("imports"),
                    display_name: String::new(),
                });

                let mut import_index = 0;
                for import in s.into_iter_with_offsets() {
                    let (offset, import) = import?;

                    if import_index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }
                    import_index += 1;

                    match import.ty {
                        TypeRef::Func(_) => {
                            items.push(Item {
                                range: Range {
                                    start: offset as u32,
                                    end: offset as u32,
                                },
                                raw_name: format!("func {func_index}"),
                                display_name: String::new(),
                            });
                            func_index += 1
                        }
                        TypeRef::Global(_) => {
                            items.push(Item {
                                range: Range {
                                    start: offset as u32,
                                    end: offset as u32,
                                },
                                raw_name: format!("global {global_index}"),
                                display_name: String::new(),
                            });
                            global_index += 1
                        }
                        TypeRef::Memory(_) => {
                            items.push(Item {
                                range: Range {
                                    start: offset as u32,
                                    end: offset as u32,
                                },
                                raw_name: format!("memory {memory_index}"),
                                display_name: String::new(),
                            });
                            memory_index += 1
                        }
                        TypeRef::Table(_) => {
                            items.push(Item {
                                range: Range {
                                    start: offset as u32,
                                    end: offset as u32,
                                },
                                raw_name: format!("table {table_index}"),
                                display_name: String::new(),
                            });
                            table_index += 1
                        }
                        TypeRef::Tag(_) => {
                            items.push(Item {
                                range: Range {
                                    start: offset as u32,
                                    end: offset as u32,
                                },
                                raw_name: format!("tag {tag_index}"),
                                display_name: String::new(),
                            });
                            tag_index += 1
                        }
                    }
                }

                if import_index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::FunctionSection(_reader) => {}
            Payload::TableSection(s) => {
                let range = s.range();
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("tables"),
                    display_name: String::new(),
                });

                let mut index = 0;
                for item in s.into_iter_with_offsets() {
                    let (offset, _) = item?;

                    if index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }

                    items.push(Item {
                        range: Range {
                            start: offset as u32,
                            end: offset as u32,
                        },
                        raw_name: format!("table {table_index}"),
                        display_name: String::new(),
                    });

                    index += 1;
                    table_index += 1;
                }

                if index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::MemorySection(s) => {
                let range = s.range();
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("memories"),
                    display_name: String::new(),
                });

                let mut index = 0;
                for item in s.into_iter_with_offsets() {
                    let (offset, _) = item?;

                    if index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }

                    items.push(Item {
                        range: Range {
                            start: offset as u32,
                            end: offset as u32,
                        },
                        raw_name: format!("memory {memory_index}"),
                        display_name: String::new(),
                    });

                    index += 1;
                    memory_index += 1;
                }

                if index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::TagSection(s) => {
                let range = s.range();
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("tags"),
                    display_name: String::new(),
                });

                let mut index = 0;
                for item in s.into_iter_with_offsets() {
                    let (offset, _) = item?;

                    if index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }

                    items.push(Item {
                        range: Range {
                            start: offset as u32,
                            end: offset as u32,
                        },
                        raw_name: format!("tag {tag_index}"),
                        display_name: String::new(),
                    });

                    index += 1;
                    tag_index += 1;
                }

                if index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::GlobalSection(s) => {
                let range = s.range();
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("globals"),
                    display_name: String::new(),
                });

                let mut index = 0;
                for item in s.into_iter_with_offsets() {
                    let (offset, _) = item?;

                    if index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }

                    items.push(Item {
                        range: Range {
                            start: offset as u32,
                            end: offset as u32,
                        },
                        raw_name: format!("global {global_index}"),
                        display_name: String::new(),
                    });

                    index += 1;
                    global_index += 1;
                }

                if index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::ExportSection(s) => {
                items.push(Item {
                    range: convert_range(&s.range()),
                    raw_name: format!("exports"),
                    display_name: String::new(),
                });
            }
            Payload::StartSection { func: _, range } => {
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("start"),
                    display_name: String::new(),
                });
            }
            Payload::ElementSection(s) => {
                let range = s.range();
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("elems"),
                    display_name: String::new(),
                });

                let mut index = 0;
                for item in s.into_iter_with_offsets() {
                    let (offset, _) = item?;

                    if index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }

                    items.push(Item {
                        range: Range {
                            start: offset as u32,
                            end: offset as u32,
                        },
                        raw_name: format!("elem {elem_index}"),
                        display_name: String::new(),
                    });

                    index += 1;
                    elem_index += 1;
                }

                if index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }
            Payload::CodeSectionStart { range, .. } => {
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("funcs"),
                    display_name: String::new(),
                });
            }
            Payload::CodeSectionEntry(body) => {
                items.push(Item {
                    range: convert_range(&body.range()),
                    raw_name: format!("func {func_index}"),
                    display_name: String::new(),
                });
                func_index += 1;
            }
            Payload::DataCountSection { .. } => {}
            Payload::DataSection(s) => {
                let range = s.range();
                items.push(Item {
                    range: convert_range(&range),
                    raw_name: format!("data"),
                    display_name: String::new(),
                });

                let mut index = 0;
                for item in s.into_iter_with_offsets() {
                    let (offset, _) = item?;

                    if index != 0 {
                        items.last_mut().unwrap().range.end = offset as u32;
                    }

                    items.push(Item {
                        range: Range {
                            start: offset as u32,
                            end: offset as u32,
                        },
                        raw_name: format!("data {data_index}"),
                        display_name: String::new(),
                    });

                    index += 1;
                    data_index += 1;
                }

                if index != 0 {
                    items.last_mut().unwrap().range.end = range.end as u32;
                }
            }

            Payload::End(_) => {
                break;
            }

            _ => {}
        }
    }

    // TODO: make this less stringly typed
    let aliases: Vec<Alias> = gather_aliases(&original_bytes)?;
    let mut alias_map: HashMap<String, String> = HashMap::new();
    for alias in aliases {
        alias_map.insert(alias.item_name, alias.name);
    }

    for item in &mut items {
        item.display_name = alias_map
            .get(&item.raw_name)
            .unwrap_or_else(|| &item.raw_name)
            .to_owned();
    }

    Ok(items)
}

bindings::export!(Component with_types_in bindings);
