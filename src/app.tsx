import * as React from "react";
import { useState, useEffect, useRef, useMemo } from "react";
import { Module } from "./module.js";
import type {
  Range,
  PrintPart,
  Item,
} from "../component-built/interfaces/local-module-module.d.ts";
import {
  ResizableColumns,
  type ColumnPanel,
} from "./components/ResizableColumns.js";
import { TreeView } from "./components/TreeView.js";

const MaxBytesForRich = 100 * 1024;
const initialModule = Module.load(
  new Uint8Array([
    0, 97, 115, 109, 1, 0, 0, 0, 1, 7, 1, 96, 2, 127, 127, 1, 127, 3, 2, 1, 0,
    7, 7, 1, 3, 97, 100, 100, 0, 0, 10, 9, 1, 7, 0, 32, 0, 32, 1, 106, 11,
  ]),
);

function LoadingSpinner() {
  return (
    <div className="w-full h-full flex items-center justify-center">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
    </div>
  );
}

class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean; error?: Error }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("Error caught by boundary:", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="w-screen h-screen flex items-center justify-center bg-gray-100">
          <div className="text-center p-8 bg-white rounded-lg shadow-lg max-w-md">
            <div className="text-red-500 text-4xl mb-4">⚠️</div>
            <h2 className="text-xl font-semibold text-gray-800 mb-2">
              Something went wrong
            </h2>
            <p className="text-gray-600 mb-4">
              An unexpected error occurred while loading the application.
            </p>
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
            >
              Reload Page
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export function App() {
  return (
    <div className="w-screen h-screen">
      <ErrorBoundary>
        <React.Suspense fallback={<LoadingSpinner />}>
          <AppInner />
        </React.Suspense>
      </ErrorBoundary>
    </div>
  );
}

function AppInner() {
  const [module, setModule] = useState<Promise<Module>>(initialModule);
  const [item, setItem] = useState<Item | null>(null);
  const [offset, setOffset] = useState<number | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  let loadedModule = React.use(module);

  useEffect(() => {
    const url = new URL(window.location.href);
    url.searchParams.delete("item");
    url.searchParams.delete("offset");
    window.history.pushState({}, "", url.toString());
  }, [loadedModule]);

  useEffect(() => {
    const updateItemFromURL = () => {
      const urlParams = new URLSearchParams(window.location.search);
      const itemParam = urlParams.get("item");

      if (itemParam !== null) {
        const foundItem = loadedModule.items.find(
          (item) => item.name === itemParam,
        );
        setItem(foundItem || null);
      } else {
        setItem(null);
      }
    };

    // Update on initial load
    updateItemFromURL();

    // Listen for URL changes
    const handlePopState = () => {
      updateItemFromURL();
    };

    window.addEventListener("popstate", handlePopState);
    return () => {
      window.removeEventListener("popstate", handlePopState);
    };
  }, [loadedModule.items]);

  useEffect(() => {
    const url = new URL(window.location.href);
    if (item !== null && item.name) {
      url.searchParams.set("item", item.name);
    } else {
      url.searchParams.delete("item");
    }
    window.history.pushState({}, "", url.toString());
  }, [item]);

  useEffect(() => {
    const updateOffsetFromURL = () => {
      const urlParams = new URLSearchParams(window.location.search);
      const offsetParam = urlParams.get("offset");

      if (offsetParam !== null) {
        let parsedOffset: number;
        if (offsetParam.startsWith("0x")) {
          parsedOffset = parseInt(offsetParam.replace("0x", ""), 16);
        } else {
          parsedOffset = parseInt(offsetParam);
        }
        if (!isNaN(parsedOffset)) {
          setOffset(parsedOffset);
        } else {
          setOffset(null);
        }
      } else {
        setOffset(null);
      }
    };

    // Update on initial load
    updateOffsetFromURL();

    // Listen for URL changes
    const handlePopState = () => {
      updateOffsetFromURL();
    };

    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  useEffect(() => {
    const url = new URL(window.location.href);
    if (offset !== null) {
      url.searchParams.set("offset", `0x${offset.toString(16)}`);
    } else {
      url.searchParams.delete("offset");
    }
    window.history.pushState({}, "", url.toString());
  }, [offset]);

  useEffect(() => {
    if (offset !== null && item !== null) {
      if (offset < item.range.start || offset >= item.range.end) {
        setOffset(null);
      }
    }
  }, [item, offset]);

  useEffect(() => {
    if (item === null && offset !== null) {
      // Find the smallest item that contains the offset
      let smallestItem: Item | null = null;
      let smallestSize = Infinity;

      for (const currentItem of loadedModule.items) {
        if (
          offset >= currentItem.range.start &&
          offset < currentItem.range.end
        ) {
          const size = currentItem.range.end - currentItem.range.start;
          if (size < smallestSize) {
            smallestSize = size;
            smallestItem = currentItem;
          }
        }
      }

      if (smallestItem) {
        setItem(smallestItem);
      }
    }
  }, [item, offset, loadedModule.items]);

  const handleFileLoad = async (content: ArrayBuffer) => {
    let mod = Module.load(new Uint8Array(content));
    setModule(mod);
  };

  // Drag and drop functionality
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Only hide overlay if leaving the main container
    if (e.currentTarget === e.target) {
      setIsDragOver(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      const file = files[0];
      const reader = new FileReader();
      reader.onload = (event) => {
        const content = event.target?.result as ArrayBuffer;
        handleFileLoad(content);
      };
      reader.readAsArrayBuffer(file);
    }
  };

  const panels: ColumnPanel[] = useMemo(
    () => [
      {
        id: "wat-viewer",
        title: "Text Format",
        content: (
          <React.Suspense fallback={<LoadingSpinner />}>
            <WatViewer
              content={loadedModule}
              item={item}
              offset={offset}
              setOffset={setOffset}
            />
          </React.Suspense>
        ),
        defaultWidth: 60,
        minWidth: 20,
      },
      {
        id: "tree-view",
        title: "Navigator",
        content: (
          <TreeView
            items={loadedModule.items}
            onItemSelected={(index) => {
              if (index >= 0 && loadedModule.items[index]) {
                setItem(loadedModule.items[index]);
              }
            }}
          />
        ),
        defaultWidth: 40,
        minWidth: 15,
      },
    ],
    [loadedModule, item, offset],
  );

  return (
    <div
      className="h-screen flex flex-col font-sans bg-gray-100 relative"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <Toolbar onFileLoad={handleFileLoad} module={loadedModule} />

      <div className="flex-1 overflow-hidden">
        <ResizableColumns panels={panels} />
      </div>

      {isDragOver && (
        <div className="absolute inset-0 bg-blue-500 bg-opacity-20 flex items-center justify-center z-50 pointer-events-none">
          <div className="bg-white rounded-lg shadow-lg p-8 border-2 border-dashed border-blue-500">
            <div className="text-center">
              <div className="text-2xl text-blue-600 mb-2">📁</div>
              <div className="text-lg font-semibold text-gray-800 mb-1">
                Drop WASM file here
              </div>
              <div className="text-sm text-gray-600">
                Release to load the module
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Toolbar(props: {
  onFileLoad: (content: ArrayBuffer) => void;
  module: Module;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as ArrayBuffer;
        props.onFileLoad(content);
      };
      reader.readAsArrayBuffer(file);
    }
  };

  return (
    <div className="flex items-center px-4 py-3 bg-gray-50 border-b border-gray-200 gap-4 shadow-sm">
      <div className="flex items-center gap-2">
        <WasmLogo />
        <span className="text-lg font-semibold text-gray-800">
          WebAssembly Explorer
        </span>
      </div>

      <div className="flex-1" />

      <button
        onClick={() => fileInputRef.current?.click()}
        className="px-4 py-2 bg-blue-600 text-white border-none rounded cursor-pointer text-sm font-medium hover:bg-blue-700 transition-colors"
      >
        Open
      </button>
      <input
        ref={fileInputRef}
        type="file"
        accept=".wasm,.wat"
        onChange={handleFileSelect}
        className="hidden"
      />
    </div>
  );
}

function WasmLogo() {
  return (
    <img
      src="./logo.svg"
      alt="WebAssembly Logo"
      width="32"
      height="32"
      className="text-blue-600"
    />
  );
}

function renderRichPrint(parts: PrintPart[], selectedOffset: number | null) {
  let root = document.createElement("span");
  let offsets = document.createElement("div");

  let stack = [root];
  for (let part of parts) {
    switch (part.tag) {
      case "str": {
        stack[0].append(part.val);
        break;
      }
      case "new-line": {
        stack[0].append("\n");

        let offset = document.createElement("div");
        offset.innerText = `0x${part.val.toString(16)}`;
        offset.className = "print-newline";
        offset.setAttribute("data-offset", "" + part.val);
        if (part.val == selectedOffset) {
          offset.classList.add("print-newline-selected");
        }
        offsets.append(offset);
        break;
      }
      case "name":
      case "literal":
      case "keyword":
      case "type":
      case "comment": {
        let ele = document.createElement("span");
        ele.className = "print-" + part.tag;
        stack[0].append(ele);
        stack.unshift(ele);
        break;
      }
      case "reset": {
        stack.shift();
        break;
      }
      default: {
        // @ts-expect-error the above should be exhaustive
        type remaining = typeof part.tag;
        console.error("got unknown print part", part);
      }
    }
  }

  return [root, offsets];
}

const emptyRange = { start: 0, end: 0 };

export function WatViewer({
  content,
  item,
  offset,
  setOffset,
}: {
  content: Module;
  item: Item | null;
  offset: number | null;
  setOffset: (offset: number) => void;
}) {
  let [bodyState, setBodyState] = useState<Promise<PrintPart[]>>(() =>
    Promise.resolve([]),
  );

  useEffect(() => {
    if (!item) {
      return;
    }

    let range = item.range;
    let items = (async () => {
      if (range.end - range.start > MaxBytesForRich) {
        let part: PrintPart = {
          tag: "str",
          val: await content.printPlain(range),
        };
        return [part];
      } else {
        return await content.printRich(range);
      }
    })();
    setBodyState(items);
  }, [content, item]);

  let body = React.use(bodyState);

  let contents = useRef<HTMLPreElement | null>(null);
  let offsets = useRef<HTMLPreElement | null>(null);

  useEffect(() => {
    if (!contents.current || !offsets.current || !body) {
      return;
    }
    let [newContents, newOffsets] = renderRichPrint(body, offset);
    contents.current.innerHTML = "";
    contents.current.appendChild(newContents);
    offsets.current.innerHTML = "";
    offsets.current.appendChild(newOffsets);
  }, [body, offset]);

  useEffect(() => {
    if (!offsets.current) {
      return;
    }

    let targetElement = offsets.current.querySelector(
      `[data-offset="${offset}"]`,
    );
    if (targetElement) {
      targetElement.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [offset]);

  if (!item) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        Nothing selected yet.
      </div>
    );
  }
  return (
    <div className="flex-1 overflow-auto bg-white flex min-h-full">
      <pre
        ref={offsets}
        className="wat font-mono text-xs leading-relaxed text-gray-800 whitespace-pre-wrap break-words"
        onClick={(e) => {
          if (
            e.target &&
            e.target instanceof HTMLDivElement &&
            e.target.hasAttribute("data-offset")
          ) {
            let offsetString = e.target.getAttribute("data-offset");
            if (offsetString === null) {
              return;
            }
            let offset = parseInt(offsetString);
            if (isNaN(offset)) {
              return;
            }
            setOffset(offset);
          }
        }}
      ></pre>
      <pre
        ref={contents}
        className="wat flex-1 font-mono text-xs leading-relaxed text-gray-800 whitespace-pre-wrap break-words"
      ></pre>
    </div>
  );
}
