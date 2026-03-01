import * as vscode from "vscode";
import { WasmEditorProvider } from "./WasmEditorProvider";

export function activate(context: vscode.ExtensionContext) {
  const log = vscode.window.createOutputChannel("WASM Explorer");
  context.subscriptions.push(log);
  log.appendLine("WASM Explorer extension activated");
  context.subscriptions.push(WasmEditorProvider.register(context, log));
}

export function deactivate() {}
