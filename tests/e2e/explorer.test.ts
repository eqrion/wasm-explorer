import { test, expect } from "@playwright/test";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const addWasmPath = path.join(__dirname, "../fixtures/add.wasm");

test.describe("smoke test", () => {
  test("page loads with non-empty WAT viewer and navigator", async ({
    page,
  }) => {
    await page.goto("/");
    // Wait for the app to load past the Suspense spinner
    await expect(page.locator("pre.wat").first()).not.toBeEmpty({ timeout: 10000 });
    // Navigator tree should have at least one item
    await expect(page.locator(".select-none").first()).toBeVisible();
  });
});

test.describe("file loading via input", () => {
  test("loading add.wasm updates the navigator tree", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Trigger file input
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(addWasmPath);

    // Tree should update with items from the add module
    await expect(page.locator(".select-none").first()).toBeVisible({
      timeout: 10000,
    });
  });
});

test.describe("WAT viewer rendering", () => {
  test("selecting an item renders non-empty text with syntax highlighting", async ({
    page,
  }) => {
    await page.goto("/");
    // Wait for initial load
    await expect(page.locator("pre.wat").first()).not.toBeEmpty({ timeout: 10000 });

    // Click the first selectable tree item
    const firstItem = page.locator(".select-none .cursor-pointer").first();
    await firstItem.click();

    const contentPre = page.locator("pre.wat").nth(1);
    await expect(contentPre).not.toBeEmpty({ timeout: 5000 });

    // Syntax highlighting spans should be present
    await expect(
      page.locator(".print-keyword, .print-name").first(),
    ).toBeVisible();
  });
});

test.describe("URL-driven navigation", () => {
  test("navigating to /?item=func+0 selects the correct item", async ({
    page,
  }) => {
    await page.goto("/?item=func+0");
    // Wait for the app to load
    await page.waitForLoadState("networkidle");
    // The item should be selected (blue highlight)
    await expect(page.locator(".bg-blue-100").first()).toBeVisible({
      timeout: 10000,
    });
  });

  test("navigating to /?item=nonexistent does not crash", async ({ page }) => {
    await page.goto("/?item=nonexistent");
    await page.waitForLoadState("networkidle");
    // Page should still show without error overlay
    await expect(page.locator("#root")).toBeVisible();
  });
});

test.describe("invalid bytes error display", () => {
  test("loading invalid bytes shows validation error panel", async ({
    page,
  }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Create a fake invalid wasm file and load it
    const buffer = Buffer.from([0, 1, 2, 3]);
    const invalidWasmPath = path.join(__dirname, "../fixtures/invalid.wasm");

    // Write invalid bytes to a temp fixture
    const { writeFileSync } = await import("fs");
    writeFileSync(invalidWasmPath, buffer);

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(invalidWasmPath);

    // Validation error panel should appear
    await expect(page.locator(".bg-red-50")).toBeVisible({ timeout: 10000 });
    await expect(page.locator(".text-red-600")).not.toBeEmpty();
  });
});

test.describe("download button", () => {
  test("clicking Download triggers a file download with .wasm extension", async ({
    page,
  }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: /download/i }).click();
    const download = await downloadPromise;

    expect(download.suggestedFilename()).toMatch(/\.wasm$/);
  });
});
