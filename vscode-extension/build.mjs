import * as esbuild from "esbuild";
import { copyFile, mkdir } from "fs/promises";
import { createRequire } from "module";

const require = createRequire(import.meta.url);
const watch = process.argv.includes("--watch");

await mkdir("dist", { recursive: true });
await mkdir("media", { recursive: true });

// Copy CSS overrides
await copyFile("webview/vscode-overrides.css", "dist/vscode-overrides.css");

// Copy Tailwind output from web app build (requires `npm run build` to have run first)
await copyFile("../dist/style.css", "dist/tailwind.css");

const sharedOpts = {
  bundle: true,
  sourcemap: true,
  nodePaths: ["../node_modules"],
};

// Bundle 1: Extension host (CJS, Node.js)
await esbuild.build({
  ...sharedOpts,
  entryPoints: ["src/extension.ts"],
  outfile: "dist/extension.js",
  format: "cjs",
  platform: "node",
  external: ["vscode"],
});
console.log("Built dist/extension.js");

// Bundle 2: Webview React app (ESM, browser)
await esbuild.build({
  ...sharedOpts,
  entryPoints: ["webview/main.tsx"],
  outfile: "dist/webview.js",
  format: "esm",
  platform: "browser",
  jsx: "automatic",
  jsxImportSource: "react",
});
console.log("Built dist/webview.js");

// Bundle 3: Worker (ESM, browser)
// Uses dynamic import(self.name) to load component.js at runtime - not bundled here
await esbuild.build({
  ...sharedOpts,
  entryPoints: ["webview/Worker.ts"],
  outfile: "dist/worker.js",
  format: "esm",
  platform: "browser",
});
console.log("Built dist/worker.js");

// Bundle 4: component.js with all JS dependencies inlined
// .wasm files are copied separately below - keep URL references as-is
await esbuild.build({
  ...sharedOpts,
  entryPoints: ["../component-built/component.js"],
  outfile: "media/component.js",
  format: "esm",
  platform: "browser",
  // Disable source maps for the component bundle to keep it clean
  sourcemap: false,
});
console.log("Built media/component.js");

// Copy .wasm files alongside component.js so relative URLs resolve correctly
await copyFile(
  "../component-built/component.core.wasm",
  "media/component.core.wasm",
);
await copyFile(
  "../component-built/component.core2.wasm",
  "media/component.core2.wasm",
);
console.log("Copied .wasm files to media/");
