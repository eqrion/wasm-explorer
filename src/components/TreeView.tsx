import * as React from "react";
import { useState, useMemo, useEffect } from "react";
import type { Range, Item } from "../../component-built/interfaces/local-module-module.d.ts";

interface ItemTree {
    name: string,
    range: Range,
    index: number,
    children: ItemTree[],
}

function itemsToTree(items: Item[]): ItemTree {
    if (items.length === 0) {
        return { name: "root", range: { start: 0, end: 0 }, index: -1, children: [] };
    }

    const root: ItemTree = { 
        name: "root", 
        range: { start: 0, end: items[items.length - 1]?.range.end || 0 }, 
        index: -1, 
        children: [] 
    };

    // Build tree by nesting items based on their byte ranges
    const stack: ItemTree[] = [root];

    items.forEach((item, index) => {
        const node: ItemTree = {
            name: item.name || `item_${index}`,
            range: item.range,
            index,
            children: []
        };

        // Find the correct parent by popping stack until we find a container
        while (stack.length > 1) {
            const parent = stack[stack.length - 1];
            if (item.range.start >= parent.range.start && item.range.end <= parent.range.end) {
                break;
            }
            stack.pop();
        }

        // Add to parent and push to stack if this could contain other items
        const parent = stack[stack.length - 1];
        parent.children.push(node);
        
        // Only push to stack if this item has a meaningful range that could contain others
        if (item.range.end > item.range.start) {
            stack.push(node);
        }
    });

    return root;
}

function getAllNodePaths(node: ItemTree, currentPath: string = 'root'): string[] {
    const paths = [currentPath];
    node.children.forEach(child => {
        const childPath = currentPath === 'root' ? child.name : `${currentPath}/${child.name}`;
        paths.push(...getAllNodePaths(child, childPath));
    });
    return paths;
}

interface TreeViewProps {
    items: Item[],
    onItemSelected: (index: number) => void;
}

// A tree view of items. Item nodes can be expanded, minimized and selected.
// Includes a fuzzy search box at the top which can filter which nodes are
// visible.
export function TreeView(props: TreeViewProps) {
    const itemTree: ItemTree = useMemo(() => itemsToTree(props.items), [props.items]);
    
    const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set(['root']));
    const [selectedIndex, setSelectedIndex] = useState<number>(-1);
    const [searchText, setSearchText] = useState("");

    const SIZE_LIMIT_KB = 512;
    const SIZE_LIMIT_BYTES = SIZE_LIMIT_KB * 1024;

    // Find exact match for search text
    const exactMatchIndex = useMemo(() => {
        if (!searchText.trim()) return -1;
        const query = searchText.toLowerCase();
        return props.items.findIndex(item => 
            (item.name || '').toLowerCase() === query
        );
    }, [searchText, props.items]);

    // Expand root and first item when the tree changes
    useEffect(() => {
        if (props.items.length > 0) {
            const expandedSet = new Set(['root']);
            if (itemTree.children.length > 0) {
                expandedSet.add(itemTree.children[0].name);
            }
            setExpandedNodes(expandedSet);
        }
    }, [itemTree, props.items.length]);

    const toggleExpanded = (nodePath: string) => {
        setExpandedNodes(prev => {
            const next = new Set(prev);
            if (next.has(nodePath)) {
                next.delete(nodePath);
            } else {
                next.add(nodePath);
            }
            return next;
        });
    };

    const handleItemSelect = (index: number) => {
        const item = props.items[index];
        const sizeInBytes = item.range.end - item.range.start;
        
        if (sizeInBytes > SIZE_LIMIT_BYTES) {
            const sizeInMB = (sizeInBytes / 1024 / 1024).toFixed(1);
            const confirmed = confirm(
                `This item contains ${sizeInMB} MB of data, which may take a long time to render and make the browser unresponsive. Are you sure you want to select it?`
            );
            if (!confirmed) return;
        }
        
        setSelectedIndex(index);
        props.onItemSelected(index);
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && exactMatchIndex >= 0) {
            handleItemSelect(exactMatchIndex);
        }
    };

    const shouldShowNode = (node: ItemTree, path: string): boolean => {
        if (!searchText.trim()) return true;
        
        const query = searchText.toLowerCase();
        const name = node.name.toLowerCase();
        
        // Show if this node matches
        if (name.includes(query)) return true;
        
        // Show if any descendant matches
        const hasMatchingDescendant = (n: ItemTree): boolean => {
            return n.children.some(child => 
                child.name.toLowerCase().includes(query) || hasMatchingDescendant(child)
            );
        };
        
        return hasMatchingDescendant(node);
    };

    const renderTreeNode = (node: ItemTree, path: string, depth: number = 0): React.ReactNode => {
        const isExpanded = expandedNodes.has(path);
        const isSelected = selectedIndex === node.index;
        const isExactMatch = exactMatchIndex >= 0 && node.index === exactMatchIndex;
        const hasChildren = node.children.length > 0;
        const shouldShow = shouldShowNode(node, path);

        if (!shouldShow) return null;

        // Auto-expand nodes when searching to show matches
        const shouldAutoExpand = searchText.trim() && node.children.some(child => 
            shouldShowNode(child, path === 'root' ? child.name : `${path}/${child.name}`)
        );

        const effectivelyExpanded = isExpanded || shouldAutoExpand;

        return (
            <div key={path} className="select-none">
                {node.name !== "root" && (
                    <div
                        className={`flex items-center py-1 px-2 cursor-pointer hover:bg-gray-100 ${
                            isSelected ? 'bg-blue-100 text-blue-900' : 
                            isExactMatch ? 'bg-yellow-100 border-2 border-yellow-400 text-yellow-900' : ''
                        }`}
                        style={{ paddingLeft: `${depth * 16 + 8}px` }}
                        onClick={() => node.index >= 0 && handleItemSelect(node.index)}
                    >
                        {hasChildren && (
                            <button
                                onClick={(e) => {
                                    e.stopPropagation();
                                    toggleExpanded(path);
                                }}
                                className="mr-1 w-4 h-4 flex items-center justify-center text-gray-500 hover:text-gray-700"
                            >
                                {effectivelyExpanded ? '▼' : '▶'}
                            </button>
                        )}
                        {!hasChildren && <div className="w-5" />}
                        <span className="text-sm truncate flex-1">{node.name}</span>
                        {node.index >= 0 && (
                            <span className="text-xs text-gray-500 ml-2">
                                [0x{node.range.start.toString(16)}-0x{node.range.end.toString(16)}] ({node.range.end - node.range.start} bytes)
                            </span>
                        )}
                    </div>
                )}
                {effectivelyExpanded && hasChildren && (
                    <div>
                        {node.children.map(child => {
                            const childPath = path === 'root' ? child.name : `${path}/${child.name}`;
                            return renderTreeNode(child, childPath, depth + 1);
                        })}
                    </div>
                )}
            </div>
        );
    };

    return (
        <div className="flex flex-col h-full bg-white border-r border-gray-200">
            <div>
                <input
                    type="text"
                    placeholder="Search"
                    value={searchText}
                    onChange={(e) => setSearchText(e.target.value)}
                    onKeyDown={handleKeyDown}
                    className="w-full px-4 py-3 mb-4 bg-white border-b border-gray-100 text-md outline-none"
                />
            </div>
            <div className="flex-1 overflow-y-auto">
                {renderTreeNode(itemTree, 'root')}
            </div>
        </div>
    );
}