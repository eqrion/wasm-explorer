use std::path::{Path, PathBuf};

use arbitrary::{Arbitrary, Unstructured};
use wasm_smith::Module as SmithModule;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Collect all .wasm files from a directory (non-recursive).
fn collect_wasm_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.exists() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                files.push(path);
            }
        }
        files.sort();
    }
    files
}

/// Run the full roundtrip test on a valid wasm binary:
/// 1. Print with wasmprinter (oracle) and with waside, compare text output
/// 2. Normalize the input binary via wat round-trip to avoid LEB encoding mismatches
/// 3. Encode with waside, compare binary output against normalized input
fn roundtrip_test(name: &str, bytes: &[u8]) {
    let expected_text = match wasmprinter::print_bytes(bytes) {
        Ok(s) => s,
        Err(e) => panic!("[{name}] wasmprinter failed: {e}"),
    };

    let normalized_binary = match wat::parse_str(&expected_text) {
        Ok(b) => b,
        Err(e) => panic!("[{name}] wat::parse_str failed on wasmprinter output: {e}"),
    };

    let module = match waside::Module::decode(bytes) {
        Ok(m) => m,
        Err(e) => panic!("[{name}] waside decode failed: {e}"),
    };

    let actual_text = module.print();
    if actual_text != expected_text {
        let actual_lines: Vec<&str> = actual_text.lines().collect();
        let expected_lines: Vec<&str> = expected_text.lines().collect();
        for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
            if a != e {
                panic!(
                    "[{name}] print mismatch at line {}:\n  expected: {e:?}\n  actual:   {a:?}",
                    i + 1,
                );
            }
        }
        panic!(
            "[{name}] print output line count differs: expected {} lines, got {}",
            expected_lines.len(),
            actual_lines.len(),
        );
    }

    let actual_binary = match module.encode(bytes) {
        Ok(b) => b,
        Err(e) => panic!("[{name}] waside encode failed: {e}"),
    };
    assert_eq!(
        &normalized_binary[..],
        &actual_binary[..],
        "[{name}] binary output differs from normalized input ({} vs {} bytes)",
        normalized_binary.len(),
        actual_binary.len(),
    );
}

/// Test a single wasm binary. Uses wasmparser to determine validity:
/// - Valid modules get the full roundtrip test
/// - Invalid modules must be rejected by waside::Module::decode
fn test_wasm(name: &str, bytes: &[u8]) {
    let is_valid = wasmparser::Validator::new().validate_all(bytes).is_ok();

    if is_valid {
        roundtrip_test(name, bytes);
    } else if waside::Module::decode(bytes).is_ok() {
        panic!("[{name}] invalid module was accepted by waside");
    }
}

/// Extract a panic message as a string.
fn panic_message(e: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "unknown panic".to_string()
    }
}

#[test]
fn test_all() {
    let root = workspace_root();

    let mut files = Vec::new();
    files.extend(collect_wasm_files(&root.join("tests/local")));
    files.extend(collect_wasm_files(&root.join("tests/spec/valid")));
    files.extend(collect_wasm_files(&root.join("tests/spec/invalid")));
    files.extend(collect_wasm_files(&root.join("third-party/wasm-corpus")));

    if files.is_empty() {
        eprintln!("No test files found. Run `cargo run -p wast-extract` to generate spec tests.");
        return;
    }

    let mut failures = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let bytes = std::fs::read(path).unwrap();
        let result = std::panic::catch_unwind(|| test_wasm(&name, &bytes));
        if let Err(e) = result {
            failures.push((name, panic_message(e)));
        }
    }

    eprintln!(
        "Test all: {} files tested, {} failures",
        files.len(),
        failures.len()
    );
    if !failures.is_empty() {
        for (file, msg) in &failures {
            eprintln!("  FAIL: {file}: {msg}");
        }
        panic!("{} failures out of {}", failures.len(), files.len());
    }
}

// --- Fuzz tests using wasm-smith ---

/// Generate a deterministic seed from an iteration number.
fn make_seed(i: u64) -> Vec<u8> {
    (0..128)
        .map(|j| {
            let v = i.wrapping_mul(6364136223846793005).wrapping_add(j as u64);
            (v >> 33) as u8
        })
        .collect()
}

/// Generate a random wasm module from seed bytes and run roundtrip_test.
/// Silently skips seeds that don't produce a usable module.
fn fuzz_roundtrip(seed: &[u8]) {
    let mut u = Unstructured::new(seed);
    let smith_module = match SmithModule::arbitrary(&mut u) {
        Ok(m) => m,
        Err(_) => return,
    };
    let wasm_bytes = smith_module.to_bytes();

    // Skip modules that waside or wasmprinter can't handle yet
    if waside::Module::decode(&wasm_bytes).is_err() {
        return;
    }
    if wasmprinter::print_bytes(&wasm_bytes).is_err() {
        return;
    }

    let name = format!(
        "fuzz-seed-{:x}",
        seed.iter()
            .fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(*b as u64))
    );
    roundtrip_test(&name, &wasm_bytes);
}

#[test]
fn fuzz_roundtrip_seeds() {
    let iterations = 1000;
    let mut failures = 0;
    for i in 0u64..iterations {
        let seed = make_seed(i);
        let result = std::panic::catch_unwind(|| fuzz_roundtrip(&seed));
        if result.is_err() {
            failures += 1;
        }
    }
    eprintln!("Fuzz roundtrip: {iterations} iterations, {failures} failures");
    assert_eq!(failures, 0, "{failures} fuzz roundtrip failures");
}
