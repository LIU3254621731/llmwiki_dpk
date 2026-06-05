import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import MarkdownRenderer, { parseFrontmatter } from "@/components/common/MarkdownRenderer";

describe("MarkdownRenderer", () => {
  it("renders a heading", () => {
    render(<MarkdownRenderer content="# Hello World" />);
    const elements = screen.getAllByText("Hello World");
    expect(elements.length).toBeGreaterThanOrEqual(1);
    expect(elements.some(el => el.tagName === "H1")).toBe(true);
  });

  it("renders a paragraph", () => {
    render(<MarkdownRenderer content="Some text content" />);
    expect(screen.getByText("Some text content")).toBeInTheDocument();
  });

  it("renders inline code", () => {
    render(<MarkdownRenderer content="Use `const` keyword" />);
    expect(screen.getByText("const", { selector: "code" })).toBeInTheDocument();
  });

  it("renders bold text", () => {
    render(<MarkdownRenderer content="This is **important**" />);
    const el = screen.getByText("important");
    expect(el.tagName).toBe("STRONG");
  });

  it("renders a link", () => {
    render(<MarkdownRenderer content='[Click here](https://example.com)' />);
    const link = screen.getByText("Click here");
    expect(link.tagName).toBe("A");
    expect(link.getAttribute("href")).toBe("https://example.com");
  });

  it("renders images as links", () => {
    render(<MarkdownRenderer content="![alt text](image.png)" />);
    const link = screen.getByText("alt text");
    expect(link.tagName).toBe("A");
    expect(link.getAttribute("href")).toBe("image.png");
  });

  it("renders wiki links", () => {
    render(<MarkdownRenderer content="See [[MyPage]] for details" />);
    expect(screen.getByText("MyPage")).toBeInTheDocument();
  });

  it("hides frontmatter by default", () => {
    const content = `---
title: Test
---
# Heading
Some content`;
    render(<MarkdownRenderer content={content} />);
    const headings = screen.getAllByText("Heading");
    expect(headings.some(el => el.tagName === "H1")).toBe(true);
    expect(screen.getByText("Some content")).toBeInTheDocument();
    expect(screen.queryByText("title:")).not.toBeInTheDocument();
  });
});

describe("parseFrontmatter()", () => {
  it("parses frontmatter keys", () => {
    const { frontmatter, body } = parseFrontmatter(`---
title: Hello
tags: [a, b, c]
---
Body text`);
    expect(frontmatter.title).toBe("Hello");
    expect(frontmatter.tags).toEqual(["a", "b", "c"]);
    expect(body).toBe("Body text");
  });

  it("returns empty frontmatter when no frontmatter is present", () => {
    const { frontmatter, body } = parseFrontmatter("# Just content");
    expect(frontmatter).toEqual({});
    expect(body).toBe("# Just content");
  });
});
