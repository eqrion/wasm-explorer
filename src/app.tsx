import * as React from "react";
import { useState, useEffect, useRef, useMemo } from "react";
import { module } from '../component-built/component.js';
import type { Module as ModuleType, Range, PrintPart, Item } from "../component-built/interfaces/local-module-module.d.ts";

let Module = module.Module;

const MaxBytesForRich = 5 * 1024;

export function App() {
    const [module, setModule] = useState<ModuleType | null>(null);
    const [currentItemIndex, setCurrentItemIndex] = useState<number | null>(null);
    let items = useMemo(() => {
        if (!module) {
            return [];
        }
        return module.items();
    }, [module]);
    let range = currentItemIndex ? items[currentItemIndex].range : (items.length > 0 ? items[0].range : {start: 0, end: 0});

    const handleFileLoad = (content: ArrayBuffer) => {
        let mod = new Module(new Uint8Array(content));
        setModule(mod);
    };

    return (
        <div className="h-screen flex flex-col font-sans bg-gray-100">
            <Toolbar onFileLoad={handleFileLoad} items={items} setCurrentItemIndex={setCurrentItemIndex} />
            <WatViewer content={module} range={range}/>
        </div>
    );
}

// Top toolbar with actions for the application.
// Contains a 'open' button for loading a wasm file, and a search bar for
// searching for text in the disassembled wasm file.
function Toolbar(props: { 
    onFileLoad: (content: ArrayBuffer) => void;
    items: Item[];
    setCurrentItemIndex: (index: number) => void;
}) {
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [searchText, setSearchText] = useState("");
    const [showResults, setShowResults] = useState(false);
    const [highlightedIndex, setHighlightedIndex] = useState(-1);
    
    // Fuzzy search logic
    const searchResults = useMemo(() => {
        if (!searchText.trim() || props.items.length === 0) {
            return [];
        }
        
        const query = searchText.toLowerCase();
        const matches = props.items
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
    }, [searchText, props.items]);

    // Check for exact match and update current item index
    useEffect(() => {
        if (searchText.trim()) {
            const exactMatch = searchResults.find(result => 
                result.name.toLowerCase() === searchText.toLowerCase()
            );
            if (exactMatch) {
                props.setCurrentItemIndex(exactMatch.index);
            }
        }
    }, [searchText, searchResults, props.setCurrentItemIndex]);

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
        props.setCurrentItemIndex(resultIndex);
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
    content: ModuleType | null;
    range: Range,
}) {
    let body = useMemo(() => {
        if (!content) {
            return [];
        }

        if (range.end - range.start > MaxBytesForRich) {
            let part: PrintPart = { tag: 'str', val: content.printPlain(range) };
            return [part];
        } else {
            return content.printRich(range);
        }
    }, [content, range]);
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
            {content ? (
                <pre ref={contents} className="p-5 m-0 font-mono text-xs leading-relaxed text-gray-800 whitespace-pre-wrap break-words">
                </pre>
            ) : (
                <div className="flex items-center justify-center h-full text-gray-500 text-base">
                    <div className="text-center">
                        <div className="text-5xl mb-4">📄</div>
                        <div>No WebAssembly module loaded.</div>
                    </div>
                </div>
            )}
        </div>
    );
}
