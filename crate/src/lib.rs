#[macro_use]
extern crate cfg_if;

use wat;
use wasm_bindgen::prelude::*;
use web_sys::Element;
use wasmparser::*;
use anyhow::{bail, Result};
use std::fmt::Write;

cfg_if! {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        fn set_panic_hook() {}
    }
}

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

#[wasm_bindgen]
pub fn input_text(text: &str, binary: &Element, explain: &Element) {
    let (out_binary, out_explain) = run_input_text(text);
    binary.set_text_content(Some(&out_binary));
    explain.set_text_content(Some(&out_explain));
}

fn run_input_text(text: &str) -> (String, String) {
    let bytes = match wat::parse_str(&text) {
        Ok(binary) => binary,
        Err(err) => {
            return (String::new(), format!("{}", err));
        },
    };

    let mut d = Dump::new(&bytes);
    if let Err(err) = d.run() {
        return (String::new(), format!("{}", err));
    }
    (d.binary, d.explain)
}

struct Dump<'a> {
    bytes: &'a [u8],
    cur: usize,
    state: String,
    binary: String,
    explain: String,
}

const NBYTES: usize = 4;

impl<'a> Dump<'a> {
    fn new(bytes: &'a [u8]) -> Dump<'a> {
        Dump {
            bytes,
            cur: 0,
            state: String::new(),
            binary: String::new(),
            explain: String::new(),
        }
    }

    fn run(&mut self) -> Result<()> {
        let mut parser = ModuleReader::new(self.bytes)?;
        write!(self.state, "version {}", parser.get_version())?;
        self.print(parser.current_position())?;

        let mut funcs = 0;
        let mut globals = 0;
        let mut tables = 0;
        let mut memories = 0;

        while !parser.eof() {
            let section = parser.read()?;
            write!(self.state, "section {:?}", section.code)?;
            self.print(section.range().start)?;
            match section.code {
                SectionCode::Type => {
                    self.print_iter(section.get_type_section_reader()?, |me, end, i| {
                        write!(me.state, "type {:?}", i)?;
                        me.print(end)
                    })?
                }
                SectionCode::Import => {
                    self.print_iter(section.get_import_section_reader()?, |me, end, i| {
                        write!(me.state, "import ")?;
                        match i.ty {
                            ImportSectionEntryType::Function(_) => {
                                write!(me.state, "[func {}]", funcs)?;
                                funcs += 1;
                            }
                            ImportSectionEntryType::Memory(_) => {
                                write!(me.state, "[memory {}]", memories)?;
                                memories += 1;
                            }
                            ImportSectionEntryType::Table(_) => {
                                write!(me.state, "[table {}]", tables)?;
                                tables += 1;
                            }
                            ImportSectionEntryType::Global(_) => {
                                write!(me.state, "[global {}]", globals)?;
                                globals += 1;
                            }
                        }
                        write!(me.state, " {:?}", i)?;
                        me.print(end)
                    })?
                }
                SectionCode::Function => {
                    let mut cnt = 0;
                    self.print_iter(section.get_function_section_reader()?, |me, end, i| {
                        write!(me.state, "[func {}] type {:?}", cnt + funcs, i)?;
                        cnt += 1;
                        me.print(end)
                    })?
                }
                SectionCode::Table => {
                    self.print_iter(section.get_table_section_reader()?, |me, end, i| {
                        write!(me.state, "[table {}] {:?}", tables, i)?;
                        tables += 1;
                        me.print(end)
                    })?
                }
                SectionCode::Memory => {
                    self.print_iter(section.get_memory_section_reader()?, |me, end, i| {
                        write!(me.state, "[memory {}] {:?}", memories, i)?;
                        memories += 1;
                        me.print(end)
                    })?
                }
                SectionCode::Export => {
                    self.print_iter(section.get_export_section_reader()?, |me, end, i| {
                        write!(me.state, "export {:?}", i)?;
                        me.print(end)
                    })?
                }
                SectionCode::Global => {
                    self.print_iter(section.get_global_section_reader()?, |me, _end, i| {
                        write!(me.state, "[global {}] {:?}", globals, i.ty)?;
                        globals += 1;
                        me.print(i.init_expr.get_binary_reader().original_position())?;
                        me.print_ops(i.init_expr.get_operators_reader())
                    })?
                }
                SectionCode::Start => {
                    let start = section.get_start_section_content()?;
                    write!(self.state, "start function {}", start)?;
                    self.print(section.range().end)?;
                }
                SectionCode::DataCount => {
                    let start = section.get_data_count_section_content()?;
                    write!(self.state, "data count {}", start)?;
                    self.print(section.range().end)?;
                }
                SectionCode::Element => {
                    self.print_iter(section.get_element_section_reader()?, |me, _end, i| {
                        write!(me.state, "element {:?}", i.ty)?;
                        let mut items = i.items.get_items_reader()?;
                        match i.kind {
                            ElementKind::Passive => {
                                write!(me.state, " passive, {} items", items.get_count())?;
                            }
                            ElementKind::Active {
                                table_index,
                                init_expr,
                            } => {
                                write!(me.state, " table[{}]", table_index)?;
                                me.print(init_expr.get_binary_reader().original_position())?;
                                me.print_ops(init_expr.get_operators_reader())?;
                                write!(me.state, "{} items", items.get_count())?;
                            }
                            ElementKind::Declared => {
                                write!(me.state, " declared {} items", items.get_count())?;
                            }
                        }
                        me.print(items.original_position())?;
                        for _ in 0..items.get_count() {
                            let item = items.read()?;
                            write!(me.state, "item {:?}", item)?;
                            me.print(items.original_position())?;
                        }
                        Ok(())
                    })?
                }

                SectionCode::Data => {
                    self.print_iter(section.get_data_section_reader()?, |me, end, i| {
                        match i.kind {
                            DataKind::Passive => {
                                write!(me.state, "data passive")?;
                                me.print(end - i.data.len())?;
                            }
                            DataKind::Active {
                                memory_index,
                                init_expr,
                            } => {
                                write!(me.state, "data memory[{}]", memory_index)?;
                                me.print(init_expr.get_binary_reader().original_position())?;
                                me.print_ops(init_expr.get_operators_reader())?;
                            }
                        }
                        write!(me.binary, "0x{:04x} |", me.cur)?;
                        for _ in 0..NBYTES {
                            write!(me.binary, "---")?;
                        }
                        write!(me.binary, "-| ... {} bytes of data\n", i.data.len())?;
                        me.cur = end;
                        Ok(())
                    })?
                }

                SectionCode::Code => {
                    self.print_iter(section.get_code_section_reader()?, |me, _end, i| {
                        write!(
                            me.binary,
                            "============== func {} ====================\n",
                            funcs
                        )?;
                        funcs += 1;
                        write!(me.state, "size of function")?;
                        me.print(i.get_binary_reader().original_position())?;
                        let mut locals = i.get_locals_reader()?;
                        write!(me.state, "{} local blocks", locals.get_count())?;
                        me.print(locals.original_position())?;
                        for _ in 0..locals.get_count() {
                            let (amt, ty) = locals.read()?;
                            write!(me.state, "{} locals of type {:?}", amt, ty)?;
                            me.print(locals.original_position())?;
                        }
                        me.print_ops(i.get_operators_reader()?)?;
                        Ok(())
                    })?
                }

                SectionCode::Custom { .. } => {
                    write!(self.binary, "0x{:04x} |", self.cur)?;
                    for _ in 0..NBYTES {
                        write!(self.binary, "---")?;
                    }
                    write!(
                        self.binary,
                        "-| ... {} bytes of data\n",
                        section.get_binary_reader().bytes_remaining()
                    )?;
                    self.cur = section.range().end;
                }
            }
        }

        assert_eq!(self.cur, self.bytes.len());
        Ok(())
    }

    fn print_iter<T>(
        &mut self,
        mut iter: T,
        mut print: impl FnMut(&mut Self, usize, T::Item) -> Result<()>,
    ) -> Result<()>
    where
        T: SectionReader + SectionWithLimitedItems,
    {
        write!(self.state, "{} count", iter.get_count())?;
        self.print(iter.original_position())?;
        for _ in 0..iter.get_count() {
            let item = iter.read()?;
            print(self, iter.original_position(), item)?;
        }
        if !iter.eof() {
            bail!("too many bytes in section");
        }
        Ok(())
    }

    fn print_ops(&mut self, mut i: OperatorsReader) -> Result<()> {
        while !i.eof() {
            match i.read() {
                Ok(op) => write!(self.state, "{:?}", op)?,
                Err(_) => write!(self.state, "??")?,
            }
            self.print(i.original_position())?;
        }
        Ok(())
    }

    fn print(&mut self, end: usize) -> Result<()> {
        assert!(self.cur < end);
        let bytes = &self.bytes[self.cur..end];
        write!(self.binary, "0x{:04x} |", self.cur)?;
        for (i, chunk) in bytes.chunks(NBYTES).enumerate() {
            if i > 0 {
                self.binary.push_str("       |");
                self.explain.push_str("");
            }
            for j in 0..NBYTES {
                match chunk.get(j) {
                    Some(b) => write!(self.binary, " {:02x}", b)?,
                    None => write!(self.binary, "   ")?,
                }
            }
            if i == 0 {
                self.explain.push_str(&self.state);
                self.state.truncate(0);
            }
            self.explain.push_str("\n");
            self.binary.push_str("\n");
        }
        self.cur = end;
        Ok(())
    }
}
