import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import App from "@/App";

// Mock the dynamic import
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  listen: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("App", () => {
  it("renders the loading state initially", async () => {
    render(
      <MemoryRouter>
        <App />
      </MemoryRouter>
    );
    expect(screen.getByText("加载中...")).toBeInTheDocument();
  });
});
