#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasm_smith::Module;

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let module = match Module::arbitrary(&mut u) {
        Ok(m) => m,
        Err(_) => return,
    };
    let wasm = module.to_bytes();

    let ast = match waside::Module::decode(&wasm) {
        Ok(m) => m,
        Err(_) => return,
    };

    let encoded = ast.encode(&wasm).expect("encode must not fail on decoded module");

    let ast2 = waside::Module::decode(&encoded)
        .expect("re-decode must not fail on encoded module");

    assert_eq!(ast, ast2, "roundtrip mismatch");
});
