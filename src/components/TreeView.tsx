import * as React from "react";
import { useState, useMemo, useEffect, useRef } from "react";
import type {
  Range,
  Item,
} from "../../component-built/interfaces/local-module-module.d.ts";

interface ItemTree {
  name: string;
  range: Range;
  index: number;
  children: ItemTree[];
}

function calculateSimilarityScore(
  query: string,
  itemName: string,
  itemRange?: Range,
): number {
  if (!itemName) return 0;

  const queryLower = query.toLowerCase();
  const nameLower = itemName.toLowerCase();

  // Check if query is a hexadecimal number
  if (queryLower.startsWith("0x") && itemRange) {
    const hexValue = parseInt(queryLower.replace("0x", ""), 16);
    if (!isNaN(hexValue)) {
      // Check if the hex value is within this item's range
      if (hexValue >= itemRange.start && hexValue <= itemRange.end) {
        const rangeSize = itemRange.end - itemRange.start;
        // Give higher score for smaller ranges (more specific matches)
        // Use inverse relationship: smaller ranges get higher scores
        return Math.max(0.5, 1 - rangeSize / 1000000); // Cap minimum at 0.5, scale by MB
      }
      return 0; // No match if hex value is outside range
    }
  }

  // Calculate similarity score (simple character overlap)
  const commonChars = nameLower
    .split("")
    .filter((char) => queryLower.includes(char)).length;
  const maxLength = Math.max(nameLower.length, queryLower.length);
  const score = commonChars / maxLength;

  // Boost score for exact substring matches
  if (nameLower.includes(queryLower)) {
    const substringBoost = queryLower.length / nameLower.length;
    return Math.min(score + substringBoost, 1);
  }

  return score;
}

function itemsToTree(items: Item[]): ItemTree {
  if (items.length === 0) {
    return {
      name: "root",
      range: { start: 0, end: 0 },
      index: -1,
      children: [],
    };
  }

  const root: ItemTree = {
    name: "root",
    range: { start: 0, end: items[items.length - 1]?.range.end || 0 },
    index: -1,
    children: [],
  };

  // Build tree by nesting items based on their byte ranges
  const stack: ItemTree[] = [root];

  items.forEach((item, index) => {
    const node: ItemTree = {
      name: item.name || `item_${index}`,
      range: item.range,
      index,
      children: [],
    };

    // Find the correct parent by popping stack until we find a container
    while (stack.length > 1) {
      const parent = stack[stack.length - 1];
      if (
        item.range.start >= parent.range.start &&
        item.range.end <= parent.range.end
      ) {
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

interface TreeViewProps {
  items: Item[];
  selectedItem: number | null;
  onItemSelected: (index: number) => void;
}

// A tree view of items. Item nodes can be expanded, minimized and selected.
// Includes a fuzzy search box at the top which can filter which nodes are
// visible.
export function TreeView(props: TreeViewProps) {
  const itemTree: ItemTree = useMemo(
    () => itemsToTree(props.items),
    [props.items],
  );

  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(
    new Set(["root"]),
  );
  const [selectedIndex, setSelectedIndex] = useState<number>(-1);
  const [searchText, setSearchText] = useState("");
  const selectedNodeRef = useRef<HTMLDivElement>(null);

  const SIZE_LIMIT_KB = 512;
  const SIZE_LIMIT_BYTES = SIZE_LIMIT_KB * 1024;

  // Find closest match for search text using similarity scoring
  const closestMatchIndex = useMemo(() => {
    if (!searchText.trim()) return -1;
    const query = searchText.toLowerCase();

    let bestMatch = -1;
    let bestScore = 0;
    const threshold = 0.3; // Minimum similarity threshold

    props.items.forEach((item, index) => {
      const score = calculateSimilarityScore(
        query,
        item.name || "",
        item.range,
      );
      if (score > bestScore && score >= threshold) {
        bestScore = score;
        bestMatch = index;
      }
    });

    return bestMatch;
  }, [searchText, props.items]);

  // Expand root and first item when the tree changes
  useEffect(() => {
    if (props.items.length > 0) {
      const expandedSet = new Set(["root"]);
      if (itemTree.children.length > 0) {
        expandedSet.add(itemTree.children[0].name);
      }
      setExpandedNodes(expandedSet);
    }
  }, [itemTree, props.items.length]);

  // Find the path to a node with the given index
  const findNodePath = (
    node: ItemTree,
    targetIndex: number,
    currentPath: string = "root",
  ): string | null => {
    if (node.index === targetIndex) {
      return currentPath;
    }

    for (const child of node.children) {
      const childPath =
        currentPath === "root" ? child.name : `${currentPath}/${child.name}`;
      const result = findNodePath(child, targetIndex, childPath);
      if (result) return result;
    }

    return null;
  };

  // Expand all parent nodes of the selected item
  const expandPathToSelected = (targetIndex: number) => {
    if (targetIndex < 0) return;

    const targetPath = findNodePath(itemTree, targetIndex);
    if (!targetPath) return;

    setExpandedNodes((prev) => {
      const next = new Set(prev);

      // Expand all parent paths
      const pathParts = targetPath.split("/");
      for (let i = 1; i <= pathParts.length; i++) {
        const partialPath = pathParts.slice(0, i).join("/") || "root";
        next.add(partialPath);
      }

      return next;
    });
  };

  // Scroll selected item into view
  const scrollToSelected = () => {
    if (selectedNodeRef.current) {
      selectedNodeRef.current.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  };

  // Sync selectedIndex with props.selectedItem and handle expansion/scrolling
  useEffect(() => {
    if (props.selectedItem !== null && props.selectedItem !== selectedIndex) {
      setSelectedIndex(props.selectedItem);
      expandPathToSelected(props.selectedItem);
      // Delay scrolling to allow expansion to complete
      setTimeout(scrollToSelected, 100);
    }
  }, [props.selectedItem, selectedIndex, itemTree]);

  const toggleExpanded = (nodePath: string) => {
    setExpandedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(nodePath)) {
        next.delete(nodePath);
      } else {
        next.add(nodePath);
      }
      return next;
    });
  };

  const handleItemSelect = (index: number, offset?: number) => {
    const item = props.items[index];
    const sizeInBytes = item.range.end - item.range.start;

    if (sizeInBytes > SIZE_LIMIT_BYTES) {
      const sizeInMB = (sizeInBytes / 1024 / 1024).toFixed(1);
      const confirmed = confirm(
        `This item contains ${sizeInMB} MB of data, which may take a long time to render and make the browser unresponsive. Are you sure you want to select it?`,
      );
      if (!confirmed) return;
    }

    setSelectedIndex(index);
    props.onItemSelected(index);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && closestMatchIndex >= 0) {
      // TODO: pass the hexadecimal offset from the search query along
      handleItemSelect(closestMatchIndex);
    }
  };

  const shouldShowNode = (node: ItemTree, path: string): boolean => {
    if (!searchText.trim()) return true;

    const query = searchText.toLowerCase();
    const name = node.name.toLowerCase();

    // Check if search text is a hexadecimal number with 0x prefix
    if (query.startsWith("0x")) {
      const hexValue = parseInt(query.replace("0x", ""), 16);
      if (!isNaN(hexValue)) {
        // Check if the hex value is within this node's range
        if (hexValue >= node.range.start && hexValue <= node.range.end) {
          return true;
        }
      }
    }

    // Show if this node matches
    if (name.includes(query)) return true;

    // Show if any descendant matches
    const hasMatchingDescendant = (n: ItemTree): boolean => {
      return n.children.some(
        (child) =>
          child.name.toLowerCase().includes(query) ||
          hasMatchingDescendant(child),
      );
    };

    return hasMatchingDescendant(node);
  };

  const renderTreeNode = (
    node: ItemTree,
    path: string,
    depth: number = 0,
  ): React.ReactNode => {
    const isExpanded = expandedNodes.has(path);
    const isSelected = selectedIndex === node.index;
    const isClosestMatch =
      closestMatchIndex >= 0 && node.index === closestMatchIndex;
    const hasChildren = node.children.length > 0;
    const shouldShow = shouldShowNode(node, path);

    if (!shouldShow) return null;

    // Auto-expand nodes when searching to show matches
    const shouldAutoExpand =
      searchText.trim() &&
      node.children.some((child) =>
        shouldShowNode(
          child,
          path === "root" ? child.name : `${path}/${child.name}`,
        ),
      );

    const effectivelyExpanded = isExpanded || shouldAutoExpand;

    return (
      <div key={path} className="select-none">
        {node.name !== "root" && (
          <div
            ref={isSelected ? selectedNodeRef : null}
            className={`flex items-center py-1 px-2 cursor-pointer hover:bg-gray-100 ${
              isSelected
                ? "bg-blue-100 text-blue-900"
                : isClosestMatch
                  ? "bg-yellow-100 border-2 border-yellow-400 text-yellow-900"
                  : ""
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
                {effectivelyExpanded ? "▼" : "▶"}
              </button>
            )}
            {!hasChildren && <div className="w-5" />}
            <span className="text-sm truncate flex-1">{node.name}</span>
            {node.index >= 0 && (
              <span className="text-xs text-gray-500 ml-2">
                [0x{node.range.start.toString(16)}-0x
                {node.range.end.toString(16)}] (
                {node.range.end - node.range.start} bytes)
              </span>
            )}
          </div>
        )}
        {effectivelyExpanded && hasChildren && (
          <div>
            {node.children.map((child) => {
              const childPath =
                path === "root" ? child.name : `${path}/${child.name}`;
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
        {renderTreeNode(itemTree, "root")}
      </div>
    </div>
  );
}
