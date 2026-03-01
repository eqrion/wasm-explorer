import * as vscode from "vscode";
import * as fs from "fs/promises";

export class WasmEditorProvider
  implements vscode.CustomReadonlyEditorProvider
{
  public static readonly viewType = "wasmExplorer.watView";

  static register(
    context: vscode.ExtensionContext,
    log: vscode.OutputChannel,
  ): vscode.Disposable {
    return vscode.window.registerCustomEditorProvider(
      WasmEditorProvider.viewType,
      new WasmEditorProvider(context, log),
      { supportsMultipleEditorsPerDocument: false },
    );
  }

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly log: vscode.OutputChannel,
  ) {}

  async openCustomDocument(uri: vscode.Uri): Promise<vscode.CustomDocument> {
    this.log.appendLine(`openCustomDocument: ${uri.fsPath}`);
    return { uri, dispose: () => {} };
  }

  async resolveCustomEditor(
    document: vscode.CustomDocument,
    webviewPanel: vscode.WebviewPanel,
  ): Promise<void> {
    this.log.appendLine(`resolveCustomEditor: ${document.uri.fsPath}`);
    const webview = webviewPanel.webview;

    webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.context.extensionUri, "dist"),
        vscode.Uri.joinPath(this.context.extensionUri, "media"),
      ],
    };

    webview.html = this.getHtmlForWebview(webview);

    webview.onDidReceiveMessage(async (msg) => {
      if (msg.type === "ready") {
        this.log.appendLine(`webview ready, loading: ${document.uri.fsPath}`);
        try {
          const bytes = await fs.readFile(document.uri.fsPath);
          this.log.appendLine(`sending ${bytes.byteLength} bytes to webview`);
          webview.postMessage({ type: "loadFile", bytes: Array.from(bytes) });
        } catch (err) {
          this.log.appendLine(`error reading file: ${err}`);
        }
      } else if (msg.type === "log") {
        this.log.appendLine(`[webview] ${msg.text}`);
      }
    });
  }

  private getHtmlForWebview(webview: vscode.Webview): string {
    const webviewJsUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, "dist", "webview.js"),
    );
    const workerJsUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, "dist", "worker.js"),
    );
    const componentJsUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, "media", "component.js"),
    );
    const tailwindCssUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, "dist", "tailwind.css"),
    );
    const overridesCssUri = webview.asWebviewUri(
      vscode.Uri.joinPath(
        this.context.extensionUri,
        "dist",
        "vscode-overrides.css",
      ),
    );
    const nonce = generateNonce();

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy"
    content="default-src 'none';
             script-src 'nonce-${nonce}' ${webview.cspSource} 'wasm-unsafe-eval';
             style-src ${webview.cspSource} 'unsafe-inline';
             worker-src ${webview.cspSource} blob:;
             connect-src ${webview.cspSource};">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" href="${tailwindCssUri}">
  <link rel="stylesheet" href="${overridesCssUri}">
  <style nonce="${nonce}">
    html, body, #root { height: 100%; margin: 0; padding: 0; overflow: hidden; }
  </style>
</head>
<body>
  <div id="root"></div>
  <script nonce="${nonce}">
    window.__WORKER_URL__ = ${JSON.stringify(workerJsUri.toString())};
    window.__COMPONENT_JS_URL__ = ${JSON.stringify(componentJsUri.toString())};
  </script>
  <script nonce="${nonce}" type="module" src="${webviewJsUri}"></script>
</body>
</html>`;
  }
}

function generateNonce(): string {
  let text = "";
  const chars =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  for (let i = 0; i < 32; i++) {
    text += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return text;
}
