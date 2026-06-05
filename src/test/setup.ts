import { expect, vi } from "vitest";
import * as matchers from "@testing-library/jest-dom/matchers";
expect.extend(matchers);

// @antv/x6 is ESM-only and incompatible with the vitest CJS test environment.
// Mock it globally so transitive imports don't crash the test runner.
vi.mock("@antv/x6", () => ({
  Graph: vi.fn(),
  Shape: { HTML: { register: vi.fn() }, Edge: vi.fn() },
  Scroller: vi.fn(),
  Snapline: vi.fn(),
  Selection: vi.fn(),
  Keyboard: vi.fn(),
  History: vi.fn(),
}));
