import * as React from "react";
import { useState, useEffect, useRef, useMemo } from "react";
import { Module } from './module.js';
import type { Range, PrintPart, Item } from "../component-built/interfaces/local-module-module.d.ts";
import { createEmitAndSemanticDiagnosticsBuilderProgram } from "typescript";

const MaxBytesForRich = 5 * 1024;

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
        console.error('Error caught by boundary:', error, errorInfo);
    }

    render() {
        if (this.state.hasError) {
            return (
                <div className="w-screen h-screen flex items-center justify-center bg-gray-100">
                    <div className="text-center p-8 bg-white rounded-lg shadow-lg max-w-md">
                        <div className="text-red-500 text-4xl mb-4">⚠️</div>
                        <h2 className="text-xl font-semibold text-gray-800 mb-2">Something went wrong</h2>
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
                <React.Suspense fallback={<LoadingSpinner/>}>
                    <AppInner/>
                </React.Suspense>
            </ErrorBoundary>
        </div>
    );
}

const initialModule = Module.load(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 7, 1, 96, 2, 127, 127, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, 97, 100, 100, 0, 0, 10, 9, 1, 7, 0, 32, 0, 32, 1, 106, 11]));

function AppInner() {
    const [module, setModule] = useState<Promise<Module>>(initialModule);
    const [range, setRange] = useState<Range>(() => ({start: 0, end: 0}));
    const [isDragOver, setIsDragOver] = useState(false);

    const handleFileLoad = async (content: ArrayBuffer) => {
        let mod = Module.load(new Uint8Array(content));
        setModule(mod);
    };
    let loadedModule = React.use(module);

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

    return (
        <div 
            className="h-screen flex flex-col font-sans bg-gray-100 relative"
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
        >
            <Toolbar onFileLoad={handleFileLoad} module={loadedModule} setRange={setRange} />
            <React.Suspense fallback={<LoadingSpinner/>}>
                <WatViewer content={loadedModule} range={range}/>
            </React.Suspense>
            
            {isDragOver && (
                <div className="absolute inset-0 bg-blue-500 bg-opacity-20 flex items-center justify-center z-50 pointer-events-none">
                    <div className="bg-white rounded-lg shadow-lg p-8 border-2 border-dashed border-blue-500">
                        <div className="text-center">
                            <div className="text-2xl text-blue-600 mb-2">📁</div>
                            <div className="text-lg font-semibold text-gray-800 mb-1">Drop WASM file here</div>
                            <div className="text-sm text-gray-600">Release to load the module</div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

// Top toolbar with actions for the application.
// Contains a 'open' button for loading a wasm file, and a search bar for
// searching for text in the disassembled wasm file.
function Toolbar(props: { 
    onFileLoad: (content: ArrayBuffer) => void;
    module: Module;
    setRange: (range: Range) => void;
}) {
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [searchText, setSearchText] = useState("");
    const [showResults, setShowResults] = useState(false);
    const [highlightedIndex, setHighlightedIndex] = useState(-1);
    const items = props.module.items;
    useEffect(() => {
        if (items.length === 0) {
            props.setRange({start: 0, end: 0});
        } else {
            let firstItem = items[0];
            if (firstItem.name == "all" && firstItem.range.end - firstItem.range.start > MaxBytesForRich) {
                firstItem = items[1];
            }
            props.setRange(firstItem.range);
        }
    }, [items]);

    // Fuzzy search logic
    const searchResults = useMemo(() => {
        if (!searchText.trim() || items.length === 0) {
            return [];
        }
        
        const query = searchText.toLowerCase();
        const matches = items
            .map((item, index) => {
                const name = item.name?.toLowerCase() || '';
                let score = 0;
                
                // Exact match gets highest score
                if (name === query) {
                    score = 1000;
                } else if (name.includes(query)) {
                    score = 500 + (100 - name.indexOf(query));
                } else {
                    // Fuzzy matching - count matching characters in order
                    let queryIndex = 0;
                    for (let i = 0; i < name.length && queryIndex < query.length; i++) {
                        if (name[i] === query[queryIndex]) {
                            queryIndex++;
                            score += 10;
                        }
                    }
                    if (queryIndex < query.length) {
                        score = 0; // Not all query characters found
                    }
                }
                
                return { item, index, score, name: item.name || '' };
            })
            .filter(match => match.score > 0)
            .sort((a, b) => b.score - a.score)
            .slice(0, 10);
            
        return matches;
    }, [searchText, items]);

    // Check for exact match and update current item index
    useEffect(() => {
        if (searchText.trim()) {
            const exactMatch = searchResults.find(result => 
                result.name.toLowerCase() === searchText.toLowerCase()
            );
            if (exactMatch) {
                props.setRange(items[exactMatch.index].range);
            }
        }
    }, [searchText, searchResults, items]);

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

    const handleSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setSearchText(event.target.value);
        setShowResults(true);
        setHighlightedIndex(-1);
    };

    const handleResultClick = (resultName: string, resultIndex: number) => {
        setSearchText(resultName);
        setShowResults(false);
        props.setRange(items[resultIndex].range);
        setHighlightedIndex(-1);
    };

    const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
        if (!showResults || searchResults.length === 0) return;

        switch (event.key) {
            case 'ArrowDown':
                event.preventDefault();
                setHighlightedIndex(prev => 
                    prev >= searchResults.length - 1 ? 0 : prev + 1
                );
                break;
            case 'ArrowUp':
                event.preventDefault();
                setHighlightedIndex(prev => 
                    prev <= 0 ? searchResults.length - 1 : prev - 1
                );
                break;
            case 'Enter':
                event.preventDefault();
                if (highlightedIndex >= 0 && highlightedIndex < searchResults.length) {
                    const result = searchResults[highlightedIndex];
                    handleResultClick(result.name, result.index);
                }
                break;
            case 'Escape':
                setShowResults(false);
                setHighlightedIndex(-1);
                break;
        }
    };

    return (
        <div className="flex items-center px-4 py-3 bg-gray-50 border-b border-gray-200 gap-4 shadow-sm">
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
            
            <div className="flex-1" />
            
            <div className="relative">
                <input
                    type="text"
                    placeholder="Search..."
                    value={searchText}
                    onChange={handleSearchChange}
                    onKeyDown={handleKeyDown}
                    onFocus={() => setShowResults(true)}
                    onBlur={() => setTimeout(() => setShowResults(false), 200)}
                    className="px-3 py-2 bg-white border border-gray-300 rounded text-sm w-64 outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                />
                {showResults && searchResults.length > 0 && (
                    <div className="absolute top-full left-0 right-0 mt-1 bg-white border border-gray-300 rounded shadow-lg z-10 max-h-64 overflow-y-auto">
                        {searchResults.map((result, index) => (
                            <div
                                key={result.index}
                                className={`px-3 py-2 cursor-pointer text-sm border-b border-gray-100 last:border-b-0 ${
                                    index === highlightedIndex 
                                        ? 'bg-blue-100 text-blue-900' 
                                        : 'hover:bg-gray-100'
                                }`}
                                onClick={() => handleResultClick(result.name, result.index)}
                            >
                                {result.name}
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}

let isCall = /call \d+/;

function toDOM(parts: PrintPart[]) {
  let root = document.createElement('span');
  let stack = [root];
  for (let part of parts) {
    switch (part.tag) {
      case 'str': {
        stack[0].append(part.val);
        break;
      }
      case 'new-line': {
        stack[0].append("\n");
        break;
      }
      case 'name':
      case 'literal':
      case 'keyword':
      case 'type':
      case 'comment': {
        let ele = document.createElement('span');
        ele.className = 'print-' + part.tag;
        stack[0].append(ele);
        stack.unshift(ele);
        break;
      }
      case 'reset': {
        stack.shift();
        break;
      }
      default: {
        // @ts-expect-error the above should be exhaustive
        type remaining = typeof part.tag;
        console.error('got unknown print part', part);
      }
    }
  }
  return root;
}

// A large text display that will hold the 'wat' text format that has been
// loaded.
export function WatViewer({ content, range }: { 
    content: Module;
    range: Range,
}) {
    let [bodyState, setBodyState] = useState<Promise<PrintPart[]>>(() => Promise.resolve([]));
    useEffect(() => {
        let items = (async () => {
            if (range.end - range.start > MaxBytesForRich) {
                let part: PrintPart = { tag: 'str', val: await content.printPlain(range) };
                return [part];
            } else {
                return await content.printRich(range);
            }
        })();
        setBodyState(items);
    }, [content, range]);
    let body = React.use(bodyState);

    let contents = useRef<HTMLPreElement | null>(null);

    useEffect(() => {
        if (!contents.current || !body) {
            return;
        }
        let span = toDOM(body);
        contents.current.innerHTML = "";
        contents.current.appendChild(span);
    }, [body]);

    return (
        <div className="flex-1 overflow-auto bg-white">
            <pre ref={contents} className="wat p-5 m-0 font-mono text-xs leading-relaxed text-gray-800 whitespace-pre-wrap break-words">
            </pre>
        </div>
    );
}
