import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { debounce, fuzzy } from "../../src/Utilities.js";

describe("debounce", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("calls immediately on first invocation", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);
    debounced();
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("second call within window resets timer and fires once after delay", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced(); // fires immediately
    expect(fn).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(50);
    debounced(); // second call within window
    expect(fn).toHaveBeenCalledTimes(1); // not called yet

    vi.advanceTimersByTime(99);
    expect(fn).toHaveBeenCalledTimes(1); // still not called

    vi.advanceTimersByTime(1);
    expect(fn).toHaveBeenCalledTimes(2); // now fires
  });

  it("does not fire again after window if only one call was made", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced();
    expect(fn).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(200);
    expect(fn).toHaveBeenCalledTimes(1); // timer fired but !fresh check skips it
  });

  it("forwards this context and args", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);
    const ctx = { value: 42 };

    debounced.call(ctx, "a", "b");
    expect(fn).toHaveBeenCalledWith("a", "b");
  });
});

describe("fuzzy", () => {
  const items = [
    { displayName: "func add" },
    { displayName: "func multiply" },
    { displayName: "table elements" },
    { displayName: "global count" },
    { displayName: "add_helper" },
  ];

  it("empty query returns all items", () => {
    expect(fuzzy(items, "")).toEqual(items);
  });

  it("whitespace-only query returns all items", () => {
    expect(fuzzy(items, "   ")).toEqual(items);
  });

  it("unmatched query returns empty array", () => {
    expect(fuzzy(items, "zzzzzzzzz")).toEqual([]);
  });

  it("results sorted by score descending", () => {
    const result = fuzzy(items, "add");
    expect(result.length).toBeGreaterThan(0);
    // "func add" and "add_helper" should both match; they come before other items
    const names = result.map((x) => x.displayName);
    expect(names).toContain("func add");
    expect(names).toContain("add_helper");
  });

  it("consecutive char bonus makes tighter matches rank higher", () => {
    const result = fuzzy(items, "add");
    const names = result.map((x) => x.displayName);
    // "func add" has consecutive 'a','d','d' match; should rank high
    const addIdx = names.indexOf("func add");
    expect(addIdx).toBeLessThan(names.length);
  });

  it("word-boundary bonus at position 0", () => {
    // "add_helper" starts with 'a' matching 'a', word boundary at 0
    const result = fuzzy(items, "add");
    const addHelper = result.find((x) => x.displayName === "add_helper");
    expect(addHelper).toBeDefined();
  });

  it("word-boundary bonus after underscore", () => {
    const result = fuzzy(items, "helper");
    expect(result.length).toBeGreaterThan(0);
    expect(result[0].displayName).toBe("add_helper");
  });

  it("case insensitive", () => {
    const mixedItems = [{ displayName: "FuncAdd" }, { displayName: "other" }];
    const result = fuzzy(mixedItems, "funcadd");
    expect(result).toEqual([{ displayName: "FuncAdd" }]);
  });
});
