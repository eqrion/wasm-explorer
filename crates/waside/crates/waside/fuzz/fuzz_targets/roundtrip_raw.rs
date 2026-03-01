#![no_main]

use libfuzzer_sys::fuzz_target;

// Fuzz with raw bytes — catches panics on malformed input.
fuzz_target!(|data: &[u8]| {
    let ast = match waside::Module::decode(data) {
        Ok(m) => m,
        Err(_) => return,
    };

    let encoded = match ast.encode(data) {
        Ok(b) => b,
        Err(_) => return,
    };

    let ast2 = match waside::Module::decode(&encoded) {
        Ok(m) => m,
        Err(_) => panic!("re-decode failed on our own encoded output"),
    };

    assert_eq!(ast, ast2, "roundtrip mismatch");
});
