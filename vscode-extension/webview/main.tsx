import * as React from "react";
import { useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { Module } from "../../src/Module.js";
import { fuzzy } from "../../src/Utilities.js";
import type { Item } from "../../component-built/interfaces/local-module-module.js";
import { TreeView } from "../../src/components/TreeView.js";
import { WatViewer } from "../../src/components/WatViewer.js";

declare function acquireVsCodeApi(): {
  postMessage(msg: unknown): void;
  getState(): unknown;
  setState(state: unknown): void;
};

const vscode = acquireVsCodeApi();

function App() {
  const [module, setModule] = useState<Promise<Module> | null>(null);
  const [item, setItem] = useState<Item | null>(null);
  const [offset, setOffset] = useState<number | null>(null);

  useEffect(() => {
    vscode.postMessage({ type: "ready" });

    const handler = (event: MessageEvent) => {
      const msg = event.data;
      if (msg.type === "loadFile") {
        const bytes = new Uint8Array(msg.bytes);
        setModule(Module.load(bytes));
        setItem(null);
        setOffset(null);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, []);

  if (!module) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        Loading...
      </div>
    );
  }

  return (
    <React.Suspense
      fallback={
        <div className="h-full flex items-center justify-center text-gray-500">
          Decoding...
        </div>
      }
    >
      <AppInner
        module={module}
        item={item}
        setItem={setItem}
        offset={offset}
        setOffset={setOffset}
      />
    </React.Suspense>
  );
}

function AppInner({
  module: modulePromise,
  item,
  setItem,
  offset,
  setOffset,
}: {
  module: Promise<Module>;
  item: Item | null;
  setItem: (item: Item | null) => void;
  offset: number | null;
  setOffset: (offset: number | null) => void;
}) {
  const loadedModule = React.use(modulePromise);
  const validateError = loadedModule.validateError;
  const items = loadedModule.items;
  const selectedItemIndex = !item
    ? null
    : (() => {
        const idx = items.findIndex((x) => x === item);
        return idx === -1 ? null : idx;
      })();

  const searchXRef = (xref: string) => {
    const exactIdx = items.findIndex(
      (x) => x.rawName === xref || x.displayName === xref,
    );
    if (exactIdx !== -1) {
      setItem(items[exactIdx]);
      return;
    }
    const found = fuzzy(items, xref);
    if (found.length > 0) {
      setItem(found[0]);
    }
  };

  if (validateError) {
    return (
      <div className="h-full p-4 overflow-auto">
        <div className="text-red-600 font-mono text-sm">
          <div className="font-bold mb-2">
            Validation Error at 0x{validateError.offset.toString(16)}
          </div>
          <pre className="whitespace-pre-wrap">{validateError.message}</pre>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex overflow-hidden">
      <div className="w-72 flex-shrink-0 overflow-hidden flex flex-col">
        <TreeView
          items={items}
          selectedItem={selectedItemIndex}
          onItemSelected={(index, off) => {
            if (index >= 0 && items[index]) {
              setItem(items[index]);
              setOffset(off !== undefined ? off : null);
            }
          }}
        />
      </div>
      <div className="flex-1 overflow-auto">
        <WatViewer
          content={loadedModule}
          item={item}
          offset={offset}
          setOffset={(off) => setOffset(off)}
          searchXRef={searchXRef}
        />
      </div>
    </div>
  );
}

createRoot(document.getElementById("root") as Element).render(<App />);
