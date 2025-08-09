import * as React from "react";
import { useState, useEffect, useMemo } from "react";
import { createRoot } from "react-dom/client";
import { Module } from "./Module.js";
import { fuzzy } from "./Utilities.js";
import type {
  Item,
} from "../component-built/interfaces/local-module-module.js";
import {
  ResizableColumns,
  type ColumnPanel,
} from "./components/ResizableColumns.js";
import { TreeView } from "./components/TreeView.js";
import { ItemPicker } from "./components/ItemPicker.js";
import { Modal } from "./components/Modal.js";
import { Toolbar } from "./components/Toolbar.js";
import { WatViewer } from "./components/WatViewer.js";

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
  const [showItemPicker, setShowItemPicker] = useState(false);
  const [showHelpModal, setShowHelpModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<Item[]>([]);
  let loadedModule = React.use(module);
  let validateError = loadedModule.validateError;
  let items = loadedModule.items;
  let selectedItemIndex: number | null = !item
    ? null
    : items.findIndex((x) => x === item);
  if (selectedItemIndex === -1) {
    selectedItemIndex = null;
  }

  useEffect(() => {
    const url = new URL(window.location.href);
    if (
      items.length > 0 &&
      items[0].range.end - items[0].range.start <= MaxBytesForRich
    ) {
      url.searchParams.set("item", items[0].displayName);
    } else {
      url.searchParams.delete("item");
    }
    url.searchParams.delete("offset");
    window.history.pushState({}, "", url.toString());
  }, [loadedModule]);

  useEffect(() => {
    const updateItemFromURL = () => {
      const urlParams = new URLSearchParams(window.location.search);
      const itemParam = urlParams.get("item");

      if (itemParam !== null) {
        const foundItem = items.find(
          (item) =>
            item.displayName === itemParam || item.rawName === itemParam,
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
  }, [items]);

  useEffect(() => {
    const url = new URL(window.location.href);
    if (item !== null) {
      url.searchParams.set("item", item.displayName);
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

      for (const currentItem of items) {
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
  }, [item, offset, items]);

  const handleFileLoad = async (content: ArrayBuffer) => {
    let mod = Module.load(new Uint8Array(content));
    setModule(mod);
  };

  const handleDownload = async () => {
    if (!loadedModule) {
      return;
    }
    const source = await loadedModule.getSource();
    const sourceBuffer = source.buffer;
    if (!(sourceBuffer instanceof ArrayBuffer)) {
      throw new Error("Source is not an ArrayBuffer");
    }
    const blob = new Blob([sourceBuffer], { type: "application/wasm" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "module.wasm";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
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

  const searchXRef = (xref: string) => {
    // Do an exact match search
    let exactMatchIndex = items.findIndex(
      (x) => x.rawName === xref || x.displayName === xref,
    );
    if (exactMatchIndex !== -1) {
      setItem(items[exactMatchIndex]);
      return;
    }

    // Fallback to fuzzy search
    const foundItems = fuzzy(items, xref);
    if (foundItems.length === 1) {
      setItem(foundItems[0]);
    } else if (foundItems.length > 1) {
      setSearchResults(foundItems);
      setSearchQuery(xref);
      setShowItemPicker(true);
    }
  };

  const panels: ColumnPanel[] = useMemo(() => {
    if (validateError) {
      return [
        {
          id: "error-view",
          title: `Validation Error at 0x${validateError.offset.toString(16)}`,
          content: (
            <pre className="bg-red-50 p-4 text-red-600 rounded border border-red-200 whitespace-pre">
              {validateError.message}
            </pre>
          ),
          defaultWidth: 100,
          minWidth: 100,
        },
      ];
    }
    return [
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
              searchXRef={searchXRef}
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
            selectedItem={selectedItemIndex}
            onItemSelected={(index, offset) => {
              if (index >= 0 && loadedModule.items[index]) {
                setItem(loadedModule.items[index]);
                if (offset !== undefined) {
                  setOffset(offset);
                } else {
                  setOffset(null);
                }
              }
            }}
          />
        ),
        defaultWidth: 40,
        minWidth: 15,
      },
    ];
  }, [loadedModule, item, offset]);

  return (
    <div
      className="h-screen flex flex-col font-sans bg-gray-100 relative"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <Toolbar
        onFileLoad={handleFileLoad}
        onDownload={handleDownload}
        onShowHelp={() => setShowHelpModal(true)}
        module={loadedModule}
      />

      <div className="flex-1 overflow-hidden">
        <ResizableColumns panels={panels} />
      </div>

      {showItemPicker && (
        <ItemPicker
          items={searchResults}
          title={`Searching for "${searchQuery}"`}
          onSelect={(selectedItem) => {
            setItem(selectedItem);
            setShowItemPicker(false);
          }}
          onClose={() => setShowItemPicker(false)}
        />
      )}

      {showHelpModal && (
        <Modal
          // title="Help"
          onClose={() => setShowHelpModal(false)}
        >
          {/* Modal content will go here */}
          <></>
        </Modal>
      )}

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

createRoot(document.getElementById("root") as Element).render(<App />);
