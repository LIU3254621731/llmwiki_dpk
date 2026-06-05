import { describe, it, expect } from "vitest";
import { cn, formatSize, formatDateTime, formatDate } from "@/lib/utils";

describe("cn()", () => {
  it("merges class strings", () => {
    expect(cn("foo", "bar")).toBe("foo bar");
  });

  it("filters falsy values", () => {
    expect(cn("foo", false && "bar", undefined, null, "baz")).toBe("foo baz");
  });

  it("merges Tailwind conflicts via tailwind-merge", () => {
    expect(cn("px-2 py-1", "px-4")).toBe("py-1 px-4");
  });

  it("handles conditional classes", () => {
    expect(cn("base", true && "active", false && "hidden")).toBe("base active");
  });
});

describe("formatSize()", () => {
  it('returns "-" for falsy input', () => {
    expect(formatSize(0)).toBe("-");
    expect(formatSize(undefined)).toBe("-");
  });

  it("formats bytes", () => {
    expect(formatSize(500)).toBe("500 B");
  });

  it("formats kilobytes", () => {
    expect(formatSize(1500)).toBe("1.5 KB");
  });

  it("formats megabytes", () => {
    expect(formatSize(2_000_000)).toBe("1.9 MB");
  });
});

describe("formatDateTime()", () => {
  it('returns "-" for empty input', () => {
    expect(formatDateTime("")).toBe("-");
    expect(formatDateTime(undefined)).toBe("-");
  });

  it("formats an RFC3339 string", () => {
    const result = formatDateTime("2026-05-18T10:30:00+00:00");
    expect(result).toMatch(/^2026-05-18 \d{2}:\d{2}:\d{2}$/);
  });
});

describe("formatDate()", () => {
  it('returns "-" for empty input', () => {
    expect(formatDate("")).toBe("-");
    expect(formatDate(undefined)).toBe("-");
  });

  it("extracts date from an RFC3339 string", () => {
    expect(formatDate("2026-05-18T10:30:00+00:00")).toBe("2026-05-18");
  });
});
