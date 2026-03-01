import * as vscode from "vscode";
import { WasmEditorProvider } from "./WasmEditorProvider";

export function activate(context: vscode.ExtensionContext) {
  context.subscriptions.push(WasmEditorProvider.register(context));
}

export function deactivate() {}
