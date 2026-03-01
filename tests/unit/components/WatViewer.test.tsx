import * as React from "react";
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, waitFor, act, fireEvent } from "@testing-library/react";
import type { Item, PrintPart } from "../../../component-built/interfaces/local-module-module.d.ts";
import type { Module } from "../../../src/Module.js";

// Prevent Module.ts from spawning a Web Worker at import time
vi.mock("../../../src/Module.js", () => ({ Module: class {} }));

// Must be imported after mock is registered
const { WatViewer } = await import("../../../src/components/WatViewer.js");

const MaxBytesForRich = 100 * 1024;

function makeItem(start: number, end: number): Item {
  return {
    displayName: "func 0",
    rawName: "func 0",
    range: { start, end },
    definitionId: { tag: "func", val: 0 },
  };
}

function makeMockModule(
  printRich = vi.fn().mockResolvedValue([]) as unknown as Module["printRich"],
  printPlain = vi.fn().mockResolvedValue("") as unknown as Module["printPlain"],
): Module {
  return { printRich, printPlain } as unknown as Module;
}

function renderInSuspense(element: React.ReactElement) {
  return render(
    <React.Suspense fallback={<div data-testid="loading">loading</div>}>
      {element}
    </React.Suspense>,
  );
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("WatViewer", () => {
  it("renders 'Nothing selected yet.' when no item is selected", async () => {
    const mod = makeMockModule();
    await act(async () => {
      renderInSuspense(
        <WatViewer
          content={mod}
          item={null}
          offset={null}
          setOffset={vi.fn()}
          searchXRef={vi.fn()}
        />,
      );
    });
    expect(screen.getByText("Nothing selected yet.")).toBeInTheDocument();
  });

  it("calls printPlain (not printRich) for large items", async () => {
    const printRich = vi.fn().mockResolvedValue([]);
    const printPlain = vi.fn().mockResolvedValue("plain text output");
    const mod = makeMockModule(printRich as any, printPlain as any);
    const largeItem = makeItem(0, MaxBytesForRich + 1);

    await act(async () => {
      renderInSuspense(
        <WatViewer
          content={mod}
          item={largeItem}
          offset={null}
          setOffset={vi.fn()}
          searchXRef={vi.fn()}
        />,
      );
    });

    await waitFor(() => {
      expect(printPlain).toHaveBeenCalled();
    });
    expect(printRich).not.toHaveBeenCalled();
  });

  it("calls printRich (not printPlain) for small items", async () => {
    const printRich = vi.fn().mockResolvedValue([]);
    const printPlain = vi.fn().mockResolvedValue("");
    const mod = makeMockModule(printRich as any, printPlain as any);
    const smallItem = makeItem(0, MaxBytesForRich - 1);

    await act(async () => {
      renderInSuspense(
        <WatViewer
          content={mod}
          item={smallItem}
          offset={null}
          setOffset={vi.fn()}
          searchXRef={vi.fn()}
        />,
      );
    });

    await waitFor(() => {
      expect(printRich).toHaveBeenCalled();
    });
    expect(printPlain).not.toHaveBeenCalled();
  });

  it("setOffset callback fires with correct integer on offset div click", async () => {
    const setOffset = vi.fn();
    const parts: PrintPart[] = [
      { tag: "new-line", val: 42 },
      { tag: "str", val: "hello" },
    ];
    const printRich = vi.fn().mockResolvedValue(parts);
    const mod = makeMockModule(printRich as any);
    const item = makeItem(0, 100);

    await act(async () => {
      renderInSuspense(
        <WatViewer
          content={mod}
          item={item}
          offset={null}
          setOffset={setOffset}
          searchXRef={vi.fn()}
        />,
      );
    });

    await waitFor(() => {
      expect(printRich).toHaveBeenCalled();
    });

    // The offset div is imperatively created by renderRichPrint
    const offsetDiv = document.querySelector('[data-offset="42"]');
    expect(offsetDiv).not.toBeNull();

    await act(async () => {
      fireEvent.click(offsetDiv!);
    });
    expect(setOffset).toHaveBeenCalledWith(42);
  });

  it("searchXRef fires with data-xref-item value on xref anchor click", async () => {
    const searchXRef = vi.fn();
    const parts: PrintPart[] = [
      { tag: "xref", val: { tag: "func", val: 1 } },
      { tag: "str", val: "label" },
      { tag: "reset" },
    ];
    const printRich = vi.fn().mockResolvedValue(parts);
    const mod = makeMockModule(printRich as any);
    const item = makeItem(0, 100);

    await act(async () => {
      renderInSuspense(
        <WatViewer
          content={mod}
          item={item}
          offset={null}
          setOffset={vi.fn()}
          searchXRef={searchXRef}
        />,
      );
    });

    await waitFor(() => {
      expect(printRich).toHaveBeenCalled();
    });

    const anchor = document.querySelector("a[data-xref-item]");
    expect(anchor).not.toBeNull();
    const xrefValue = anchor!.getAttribute("data-xref-item");

    await act(async () => {
      fireEvent.click(anchor!);
    });
    expect(searchXRef).toHaveBeenCalledWith(xrefValue);
  });
});
