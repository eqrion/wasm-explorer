import type { Range, Item } from "../component-built/interfaces/local-module-module.js";

export interface ItemTree {
  rawName: string;
  displayName: string;
  range: Range;
  index: number;
  children: ItemTree[];
}

const ROOT_INDEX = -1;

export function itemsToTree(items: Item[]): ItemTree {
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
