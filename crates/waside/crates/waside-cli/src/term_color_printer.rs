use std::io::Write;
use termcolor::{Buffer, Color, ColorSpec, WriteColor};
use waside::{ItemId, Printer, Style};

fn color_spec(style: Style) -> ColorSpec {
    let mut spec = ColorSpec::new();
    match style {
        Style::Keyword => {
            spec.set_bold(true).set_fg(Some(Color::Blue)).set_intense(true);
        }
        Style::Type => {
            spec.set_fg(Some(Color::Green)).set_intense(true);
        }
        Style::Name => {
            spec.set_fg(Some(Color::Cyan));
        }
        Style::Literal => {
            spec.set_fg(Some(Color::Yellow)).set_intense(true);
        }
        Style::Comment => {
            spec.set_fg(Some(Color::White)).set_dimmed(true);
        }
        Style::Punctuation => {
            spec.set_dimmed(true);
        }
        Style::Default => {}
    }
    spec
}

pub struct TermColorPrinter {
    buffer: Buffer,
    indent_level: usize,
    at_line_start: bool,
    style_stack: Vec<Style>,
}

impl TermColorPrinter {
    pub fn new() -> Self {
        TermColorPrinter {
            buffer: Buffer::ansi(),
            indent_level: 0,
            at_line_start: true,
            style_stack: Vec::new(),
        }
    }

    pub fn into_output(self) -> Vec<u8> {
        self.buffer.into_inner()
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or(Style::Default)
    }
}

impl Printer for TermColorPrinter {
    fn write_str(&mut self, s: &str) {
        if self.at_line_start {
            let capped = self.indent_level.min(50);
            self.buffer.reset().unwrap();
            for _ in 0..capped {
                write!(self.buffer, "  ").unwrap();
            }
            self.at_line_start = false;
        }
        self.buffer.set_color(&color_spec(self.current_style())).unwrap();
        write!(self.buffer, "{s}").unwrap();
        self.buffer.reset().unwrap();
    }

    fn newline(&mut self, _offset: Option<usize>) {
        writeln!(self.buffer).unwrap();
        self.at_line_start = true;
    }

    fn push_style(&mut self, style: Style) {
        self.style_stack.push(style);
    }

    fn pop_style(&mut self) {
        self.style_stack.pop();
    }

    fn begin_xref(&mut self, _xref: ItemId) {}
    fn end_xref(&mut self) {}

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }
}
