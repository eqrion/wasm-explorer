mod term_color_printer;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use term_color_printer::TermColorPrinter;

#[derive(Parser)]
#[command(
    name = "waside",
    about = "WebAssembly inspection and development environment"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a wasm binary as WAT text
    Print {
        /// Path to the wasm binary file
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Print { file } => {
            let bytes = std::fs::read(&file)
                .with_context(|| format!("failed to read '{}'", file.display()))?;
            let module = waside::Module::decode(&bytes)
                .with_context(|| format!("failed to decode '{}'", file.display()))?;
            if std::io::stdout().is_terminal() {
                let mut p = TermColorPrinter::new();
                module.print_to(&mut p);
                std::io::stdout().write_all(&p.into_output())?;
            } else {
                print!("{}", module.print());
            }
        }
    }

    Ok(())
}
