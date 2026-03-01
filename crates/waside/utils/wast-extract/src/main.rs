use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        bail!("Usage: wast-extract <wast-dir> <output-dir>");
    }

    let wast_dir = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);

    let valid_dir = output_dir.join("valid");
    let invalid_dir = output_dir.join("invalid");
    std::fs::create_dir_all(&valid_dir)?;
    std::fs::create_dir_all(&invalid_dir)?;

    let mut total = 0;
    let mut valid_count = 0;
    let mut invalid_count = 0;

    visit_dir(&wast_dir, &mut |path| {
        total += 1;
        match extract_modules(path, &valid_dir, &invalid_dir) {
            Ok((valid, invalid)) => {
                valid_count += valid;
                invalid_count += invalid;
            }
            Err(e) => {
                eprintln!("Warning: failed to process {}: {}", path.display(), e);
            }
        }
    })?;

    println!(
        "Processed {total} .wast files, extracted {valid_count} valid + {invalid_count} invalid modules to {}",
        output_dir.display()
    );
    Ok(())
}

fn visit_dir(dir: &Path, cb: &mut dyn FnMut(&Path)) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_dir(&path, cb)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("wast") {
            cb(&path);
        }
    }
    Ok(())
}

fn write_module(bytes: &[u8], stem: &str, count: &mut usize, dir: &Path) -> Result<()> {
    let filename = if *count == 0 {
        format!("{stem}.wasm")
    } else {
        format!("{stem}_{count}.wasm")
    };
    std::fs::write(dir.join(&filename), bytes)?;
    *count += 1;
    Ok(())
}

fn extract_modules(
    wast_path: &Path,
    valid_dir: &Path,
    invalid_dir: &Path,
) -> Result<(usize, usize)> {
    let source = std::fs::read_to_string(wast_path)
        .with_context(|| format!("reading {}", wast_path.display()))?;

    let buf = wast::parser::ParseBuffer::new(&source).map_err(|e| anyhow::anyhow!("{}", e))?;
    let wast = wast::parser::parse::<wast::Wast>(&buf).map_err(|e| anyhow::anyhow!("{}", e))?;

    let stem = wast_path.file_stem().unwrap().to_str().unwrap();
    let mut valid_count = 0;
    let mut invalid_count = 0;

    for directive in wast.directives {
        match directive {
            wast::WastDirective::Module(mut module)
            | wast::WastDirective::ModuleDefinition(mut module) => {
                if let Ok(bytes) = module.encode() {
                    write_module(&bytes, stem, &mut valid_count, valid_dir)?;
                }
            }
            wast::WastDirective::AssertInvalid { mut module, .. } => {
                if let Ok(bytes) = module.encode() {
                    write_module(&bytes, stem, &mut invalid_count, invalid_dir)?;
                }
            }
            _ => {}
        }
    }

    Ok((valid_count, invalid_count))
}
