import * as React from "react";
import { useState, useMemo, useEffect, useRef } from "react";
import type {
  Range,
  Item,
} from "../../component-built/interfaces/local-module-module.d.ts";
import { textChangeRangeNewSpan } from "typescript";

interface ItemTree {
  rawName: string;
  displayName: string;
  range: Range;
  index: number;
  children: ItemTree[];
}

function fuzzyMatchRange(query: string, range: Range): number {
  const queryLower = query.toLowerCase();
  const hexValue = parseInt(queryLower.replace(/^0x/, ""), 16);
  if (isNaN(hexValue)) {
    return 0;
  }

  // Check if the hex value is within this item's range
  if (hexValue >= range.start && hexValue <= range.end) {
    const rangeSize = range.end - range.start;
    // Give higher score for smaller ranges (more specific matches)
    // Use inverse relationship: smaller ranges get higher scores
    return Math.max(0.5, 1 - rangeSize / 1000000); // Cap minimum at 0.5, scale by MB
  }
  return 0; // No match if hex value is outside range
}

// TODO: unify with Utilities.ts/fuzzy
function fuzzyMatchName(query: string, name: string): number {
  const queryLower = query;
  const nameLower = name.toLowerCase();

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

function fuzzyMatchItem(query: string, item: ItemTree | Item): number {
  return Math.max(
    fuzzyMatchName(query, item.displayName),
    fuzzyMatchName(query, item.rawName),
    fuzzyMatchRange(query, item.range),
  );
}

const SIZE_LIMIT_KB = 512;
const SIZE_LIMIT_BYTES = SIZE_LIMIT_KB * 1024;
const ROOT_INDEX = -1;
const MAX_SEARCH_MATCHES = 100;
const MIN_FUZZY_THRESHOLD = 0.3;

function itemsToTree(items: Item[]): ItemTree {
  if (items.length === 0) {
    return {
      rawName: "root",
      displayName: "root",
      range: { start: 0, end: 0 },
      index: -1,
      children: [],
    };
  }

  const root: ItemTree = {
    rawName: "root",
    displayName: "root",
    range: { start: 0, end: items[items.length - 1]?.range.end || 0 },
    index: ROOT_INDEX,
    children: [],
  };

  // Build tree by nesting items based on their byte ranges
  const stack: ItemTree[] = [root];

  items.forEach((item, index) => {
    const node: ItemTree = {
      rawName: item.rawName,
      displayName: item.displayName,
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

interface SearchMatch {
  score: number;
  item: ItemTree;
  ancestors: string[];
}

interface TreeViewProps {
  items: Item[];
  selectedItem: number | null;
  onItemSelected: (index: number, offset?: number) => void;
}

// A tree view of items. Item nodes can be expanded, minimized and selected.
// Includes a fuzzy search box at the top which can filter which nodes are
// visible.
export function TreeView(props: TreeViewProps) {
  const itemTree: ItemTree = useMemo(
    () => itemsToTree(props.items),
    [props.items],
  );

  const [expandedNodes, setExpandedNodes] = useState<Set<number>>(
    new Set([ROOT_INDEX]),
  );
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [searchText, setSearchText] = useState("");
  const [searchMatches, setSearchMatches] = useState<SearchMatch[]>([]);
  const [highlightedIndex, setHighlightedIndex] = useState<number | null>(null);
  const selectedNodeRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const hasSearchText = searchText.trim() !== "";

  // Focus search box when 'f' key is pressed
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (
        e.key === "f" &&
        !e.ctrlKey &&
        !e.metaKey &&
        e.target === document.body
      ) {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };

    document.addEventListener("keydown", handleGlobalKeyDown);
    return () => document.removeEventListener("keydown", handleGlobalKeyDown);
  }, []);

  // Compute search matches when search text or items change
  const computeSearchMatches = (query: string): SearchMatch[] => {
    const matches: SearchMatch[] = [];

    if (query.trim() === "") {
      return matches;
    }
    const queryLower = query.toLowerCase();

    const match = (node: ItemTree, ancestors: string[]) => {
      let newAncestors = [...ancestors, node.displayName];
      node.children.forEach((child) => {
        match(child, newAncestors);
      });

      if (node.index === ROOT_INDEX) {
        return;
      }

      // Check if we fuzzy match, but limit the number of these to a maximum size
      let score = fuzzyMatchItem(queryLower, node);

      if (score < MIN_FUZZY_THRESHOLD) {
        return;
      }

      let index = matches.findIndex((x) => score > x.score);
      if (index === -1) {
        index = matches.length;
      }

      if (index >= MAX_SEARCH_MATCHES) {
        return;
      }

      matches.splice(index, 0, {
        score,
        item: node,
        ancestors,
      });

      if (matches.length > MAX_SEARCH_MATCHES) {
        matches.pop();
      }
    };

    match(itemTree, []);

    return matches;
  };

  // Update search matches when search text changes
  useEffect(() => {
    const matches = computeSearchMatches(searchText);
    setSearchMatches(matches);
    setHighlightedIndex(matches.length > 0 ? matches[0].item.index : null);
  }, [searchText, props.items, itemTree]);

  // Get all visible items in display order for keyboard navigation
  const getVisibleItems = (): number[] => {
    const visibleItems: number[] = [];

    if (hasSearchText) {
      return searchMatches.map((x) => x.item.index);
    }

    const collectVisibleItems = (node: ItemTree) => {
      if (node.index != ROOT_INDEX) {
        visibleItems.push(node.index);
      }

      const isExpanded = expandedNodes.has(node.index);
      // (hasSearchText && searchMatched) || ;
      if (isExpanded) {
        node.children.forEach((child) => {
          collectVisibleItems(child);
        });
      }
    };
    collectVisibleItems(itemTree);

    return visibleItems;
  };

  // Expand root and first item when the tree changes
  useEffect(() => {
    if (props.items.length > 0) {
      const expandedSet = new Set([ROOT_INDEX]);
      if (itemTree.children.length > 0) {
        expandedSet.add(itemTree.children[0].index);
      }
      setExpandedNodes(expandedSet);
    }
  }, [itemTree]);

  // Find all parent nodes of a given node index
  const findParentIndices = (
    node: ItemTree,
    targetIndex: number,
    parents: number[] = [],
  ): number[] | null => {
    if (node.index === targetIndex) {
      return parents;
    }

    for (const child of node.children) {
      const result = findParentIndices(child, targetIndex, [
        ...parents,
        node.index,
      ]);
      if (result) return result;
    }

    return null;
  };

  // Expand all parent nodes of the selected item
  const expandPathToSelected = (targetIndex: number) => {
    if (targetIndex < 0) return;

    const parentIndices = findParentIndices(itemTree, targetIndex);
    if (!parentIndices) return;

    setExpandedNodes((prev) => {
      const next = new Set(prev);
      parentIndices.forEach((index) => next.add(index));
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
      setSearchText("");
      setSelectedIndex(props.selectedItem);
      expandPathToSelected(props.selectedItem);
      // Delay scrolling to allow expansion to complete
      setTimeout(scrollToSelected, 500);
    }
  }, [props.selectedItem, selectedIndex, itemTree]);

  const toggleExpanded = (nodeIndex: number) => {
    setExpandedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(nodeIndex)) {
        next.delete(nodeIndex);
      } else {
        next.add(nodeIndex);
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
    props.onItemSelected(index, offset);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (
      e.key === "Enter" &&
      highlightedIndex !== null &&
      highlightedIndex !== ROOT_INDEX
    ) {
      let offset: number | undefined = parseInt(
        searchText.replace(/^0x/, ""),
        16,
      );
      if (isNaN(offset)) {
        offset = undefined;
      }
      handleItemSelect(highlightedIndex, offset);
      return;
    }

    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const visibleItems = getVisibleItems();

      if (visibleItems.length === 0) return;

      let newIndex: number;

      if (highlightedIndex === null) {
        // No current highlight, select first item
        newIndex = visibleItems[0];
      } else {
        const currentPosition = visibleItems.indexOf(highlightedIndex);

        if (e.key === "ArrowDown") {
          // Move down, wrap to first if at end
          newIndex =
            currentPosition >= 0 && currentPosition < visibleItems.length - 1
              ? visibleItems[currentPosition + 1]
              : visibleItems[0];
        } else {
          // Move up, wrap to last if at beginning
          newIndex =
            currentPosition > 0
              ? visibleItems[currentPosition - 1]
              : visibleItems[visibleItems.length - 1];
        }
      }

      setHighlightedIndex(newIndex);

      // Expand path to highlighted item to ensure it's visible
      expandPathToSelected(newIndex);
    }
  };

  const renderTreeNode = (
    node: ItemTree,
    path: string,
    depth: number = 0,
  ): React.ReactNode => {
    const hasChildren = node.children.length > 0;
    const isExpanded = expandedNodes.has(node.index);
    const isSelected = selectedIndex === node.index;
    const isHighlighted = node.index === highlightedIndex;

    return (
      <div key={path} className="select-none">
        {node.rawName !== "root" && (
          <div
            ref={isSelected ? selectedNodeRef : null}
            className={`flex items-center py-1 px-2 cursor-pointer hover:bg-gray-100 ${
              isHighlighted
                ? "bg-yellow-100 text-yellow-900"
                : isSelected
                  ? "bg-blue-100 text-blue-900"
                  : ""
            }`}
            style={{ paddingLeft: `${depth * 16 + 8}px` }}
            onClick={() => node.index >= 0 && handleItemSelect(node.index)}
          >
            {hasChildren && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  toggleExpanded(node.index);
                }}
                className="mr-1 w-4 h-4 flex items-center justify-center text-gray-500 hover:text-gray-700"
              >
                {isExpanded ? "▼" : "▶"}
              </button>
            )}
            {!hasChildren && <div className="w-5" />}
            <span className="text-sm truncate flex-1">{node.displayName}</span>
            {node.index !== ROOT_INDEX && (
              <span className="text-xs text-gray-500 ml-2">
                [0x{node.range.start.toString(16)}-0x
                {node.range.end.toString(16)}] (
                {node.range.end - node.range.start} bytes)
              </span>
            )}
          </div>
        )}
        {isExpanded && hasChildren && (
          <div>
            {node.children.map((child) => {
              const childPath =
                path === "root" ? child.rawName : `${path}/${child.rawName}`;
              return renderTreeNode(child, childPath, depth + 1);
            })}
          </div>
        )}
      </div>
    );
  };

  const renderSearchMatch = (
    match: SearchMatch,
    index: number,
  ): React.ReactNode => {
    const isSelected = selectedIndex === match.item.index;
    const isHighlighted = match.item.index === highlightedIndex;

    return (
      <div
        key={`search-${match.item.index}-${index}`}
        className={`flex flex-col py-2 px-4 cursor-pointer hover:bg-gray-100 border-b border-gray-50 ${
          isHighlighted
            ? "bg-yellow-100 text-yellow-900"
            : isSelected
              ? "bg-blue-100 text-blue-900"
              : ""
        }`}
        onClick={() =>
          match.item.index >= 0 && handleItemSelect(match.item.index)
        }
      >
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium truncate flex-1">
            {match.item.displayName}
          </span>
          {/* {hasSearchText && (<span className="text-xs text-gray-500 ml-2">{match.score}</span>)} */}
          <span className="text-xs text-gray-500 ml-2">
            [0x{match.item.range.start.toString(16)}-0x
            {match.item.range.end.toString(16)}]
          </span>
        </div>
        <div className="text-xs text-gray-400 mt-1 truncate">
          {match.ancestors.length > 1
            ? match.ancestors.slice(1).join(" > ")
            : " "}
        </div>
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full bg-white border-r border-gray-200">
      <div>
        <input
          ref={searchInputRef}
          type="text"
          placeholder="Filter by name or binary offset"
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full px-4 py-3 mb-4 bg-white border-b border-gray-100 text-md outline-none"
        />
      </div>
      <div className="flex-1 overflow-y-auto">
        {!hasSearchText && renderTreeNode(itemTree, "root")}
        {hasSearchText && searchMatches.map((x, i) => renderSearchMatch(x, i))}
      </div>
    </div>
  );
}
