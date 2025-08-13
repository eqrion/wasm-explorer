# WebAssembly Explorer

A prototype web-based tool for exploring and analyzing WebAssembly modules. Built with React, TypeScript, and Rust using WebAssembly Components.

## Features

- **Interactive WASM Module Explorer**: Navigate through WebAssembly module structure with a tree view.
- **Rich Text Format Viewer**: View WebAssembly Text (WAT) format with syntax highlighting and semantic coloring
- **Real-time Navigation**: Click on items in the navigator to jump to specific sections in the text format
- **Cross-Reference Navigation**: Click on function names and references to navigate between related items
- **Search Functionality**: Find and navigate to specific items within modules

## Architecture

- **Frontend**: React + TypeScript application with Tailwind CSS
- **WASM Tools Integration**: Leverages `wasmparser`, `wasmprinter`, and `wat` crates for module analysis using the wasm component-model.

## Getting Started

### Prerequisites

- Node.js (for frontend build tools)
- Rust with `cargo component` installed
- `wasm32-wasip1` target for Rust

### Installation

1. Clone the repository:
```bash
git clone https://github.com/eqrion/wasm-explorer.git
cd wasm-explorer
```

2. Install dependencies:
```bash
npm install
```

3. Build the project:
```bash
npm run build
```

### Development

To start development with file watching:
```bash
npm run watch
```

This will rebuild the project automatically when files change.

## License

This project is licensed under multiple licenses:
- Apache License 2.0
- Apache License 2.0 with LLVM Exception
- MIT License

See the respective LICENSE files for details.
