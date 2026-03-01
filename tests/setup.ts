import "@testing-library/jest-dom";
import { vi } from "vitest";

// JSDOM does not implement scrollIntoView
window.HTMLElement.prototype.scrollIntoView = vi.fn();
