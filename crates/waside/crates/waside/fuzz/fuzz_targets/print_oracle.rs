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

    let expected = match wasmprinter::print_bytes(&wasm) {
        Ok(s) => s,
        Err(_) => return,
    };

    let actual = ast.print();
    assert_eq!(expected, actual, "print output mismatch with wasmprinter");
});
