use std::fmt;

pub use crate::ast::module::ItemId;

/// Text styling hints for rich output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// A language keyword (e.g. `func`, `module`).
    Keyword,
    /// A type name.
    Type,
    /// An identifier or symbolic name.
    Name,
    /// A numeric or string literal.
    Literal,
    /// A comment.
    Comment,
    /// Punctuation such as parentheses.
    Punctuation,
    /// Unstyled default text.
    Default,
}

/// Trait for outputting WAT text with optional styling and cross-references.
pub trait Printer {
    /// Write a plain string fragment.
    fn write_str(&mut self, s: &str);
    /// Emit a newline and reset to the start of a new line.
    /// `offset` is the byte offset in the wasm binary corresponding to the item
    /// being printed, if known.
    fn newline(&mut self, offset: Option<usize>);
    /// Begin a styled region. Calls may be nested; each must be paired with [`pop_style`](Printer::pop_style).
    fn push_style(&mut self, style: Style);
    /// End the most recently pushed style.
    fn pop_style(&mut self);
    /// Begin a cross-reference span. Must be paired with [`end_xref`](Printer::end_xref).
    fn begin_xref(&mut self, xref: ItemId);
    /// End the current cross-reference span.
    fn end_xref(&mut self);
    /// Increase the indentation level by one.
    fn indent(&mut self);
    /// Decrease the indentation level by one.
    fn dedent(&mut self);
}

/// A simple printer that captures plain text, ignoring styling and xrefs.
pub struct PlainTextPrinter {
    output: String,
    indent_level: usize,
    at_line_start: bool,
}

impl PlainTextPrinter {
    /// Create a new empty printer.
    pub fn new() -> Self {
        PlainTextPrinter {
            output: String::new(),
            indent_level: 0,
            at_line_start: true,
        }
    }

    /// Borrow the accumulated output as a string slice.
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Consume the printer and return the accumulated output.
    pub fn into_output(self) -> String {
        self.output
    }

    fn write_indent(&mut self) {
        if self.at_line_start {
            // Clamp indentation to match wasmprinter's MAX_NESTING_TO_PRINT
            let capped = self.indent_level.min(50);
            for _ in 0..capped {
                self.output.push_str("  ");
            }
            self.at_line_start = false;
        }
    }
}

impl Default for PlainTextPrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl Printer for PlainTextPrinter {
    fn write_str(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
    }

    fn newline(&mut self, _offset: Option<usize>) {
        self.output.push('\n');
        self.at_line_start = true;
    }

    fn push_style(&mut self, _style: Style) {}
    fn pop_style(&mut self) {}
    fn begin_xref(&mut self, _xref: ItemId) {}
    fn end_xref(&mut self) {}

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }
}

impl fmt::Write for PlainTextPrinter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Printer::write_str(self, s);
        Ok(())
    }
}
