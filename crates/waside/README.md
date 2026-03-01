# waside

A Rust library for loading WebAssembly binaries into an AST, with support for roundtrip encoding and WAT text printing. Designed for use in the wasm-explorer web application to display a virtualized, linkable, and searchable view of the wasm text format.

## Usage

```rust
// Decode a wasm binary
let bytes = std::fs::read("module.wasm")?;
let module = waside::Module::decode(&bytes)?;

// Print to WAT text
let wat = waside::print_module(&module);

// Roundtrip: encode back to binary
let encoded = module.encode()?;
```
