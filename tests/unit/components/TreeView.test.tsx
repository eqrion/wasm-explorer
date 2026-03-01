import * as React from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { itemsToTree } from "../../../src/treeUtils.js";
import { TreeView } from "../../../src/components/TreeView.js";
import type { Item } from "../../../component-built/interfaces/local-module-module.d.ts";

function makeItem(
  displayName: string,
  start: number,
  end: number,
  rawName?: string,
): Item {
  return {
    displayName,
    rawName: rawName ?? displayName,
    range: { start, end },
  };
}

// --- itemsToTree unit tests ---

describe("itemsToTree", () => {
  it("empty items returns root with no children", () => {
    const tree = itemsToTree([]);
    expect(tree.rawName).toBe("root");
    expect(tree.children).toHaveLength(0);
  });

  it("non-overlapping items become root-level children", () => {
    const items = [makeItem("func 0", 10, 50), makeItem("func 1", 60, 100)];
    const tree = itemsToTree(items);
    expect(tree.children).toHaveLength(2);
    expect(tree.children[0].displayName).toBe("func 0");
    expect(tree.children[1].displayName).toBe("func 1");
  });

  it("item B contained in item A becomes a child of A", () => {
    const items = [
      makeItem("section", 0, 200),
      makeItem("func 0", 10, 50),
    ];
    const tree = itemsToTree(items);
    expect(tree.children).toHaveLength(1);
    const section = tree.children[0];
    expect(section.displayName).toBe("section");
    expect(section.children).toHaveLength(1);
    expect(section.children[0].displayName).toBe("func 0");
  });

  it("identical ranges: both items are present", () => {
    const items = [makeItem("a", 0, 100), makeItem("b", 0, 100)];
    const tree = itemsToTree(items);
    // Both should appear somewhere in the tree
    const allNodes: string[] = [];
    const collect = (node: ReturnType<typeof itemsToTree>) => {
      allNodes.push(node.displayName);
      node.children.forEach(collect);
    };
    collect(tree);
    expect(allNodes).toContain("a");
    expect(allNodes).toContain("b");
  });
});

// --- TreeView component tests ---

describe("TreeView", () => {
  it("renders without crash with empty items", () => {
    render(
      <TreeView items={[]} selectedItem={null} onItemSelected={() => {}} />,
    );
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });

  it("renders item names", () => {
    const items = [makeItem("func 0", 10, 50), makeItem("func 1", 60, 100)];
    render(
      <TreeView items={items} selectedItem={null} onItemSelected={() => {}} />,
    );
    expect(screen.getByText("func 0")).toBeInTheDocument();
    expect(screen.getByText("func 1")).toBeInTheDocument();
  });

  it("typing in search input filters items by name", async () => {
    const user = userEvent.setup();
    const items = [
      makeItem("func add", 10, 50),
      makeItem("table elements", 60, 100),
    ];
    render(
      <TreeView items={items} selectedItem={null} onItemSelected={() => {}} />,
    );
    const input = screen.getByRole("textbox");
    await user.type(input, "func");
    expect(screen.getByText("func add")).toBeInTheDocument();
    expect(screen.queryByText("table elements")).not.toBeInTheDocument();
  });

  it("Enter key fires onItemSelected for highlighted item", async () => {
    const user = userEvent.setup();
    const onItemSelected = vi.fn();
    const items = [makeItem("func add", 10, 50), makeItem("func mul", 60, 100)];
    render(
      <TreeView
        items={items}
        selectedItem={null}
        onItemSelected={onItemSelected}
      />,
    );
    const input = screen.getByRole("textbox");
    await user.type(input, "func");
    // highlightedIndex should be set to first match; press Enter to select
    await user.keyboard("{Enter}");
    expect(onItemSelected).toHaveBeenCalled();
  });

  it("ArrowDown moves highlight to next item, Enter selects it", async () => {
    const user = userEvent.setup();
    const onItemSelected = vi.fn();
    const items = [makeItem("func add", 10, 50), makeItem("func mul", 60, 100)];
    render(
      <TreeView
        items={items}
        selectedItem={null}
        onItemSelected={onItemSelected}
      />,
    );
    const input = screen.getByRole("textbox");
    await user.type(input, "func");
    // After typing, first match is highlighted. ArrowDown moves to second.
    await user.keyboard("{ArrowDown}");
    await user.keyboard("{Enter}");
    expect(onItemSelected).toHaveBeenCalled();
    // Check that index 1 (second item) was selected
    expect(onItemSelected.mock.calls[0][0]).toBe(1);
  });
});
