import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { Icon } from "./Icon";

describe("Icon", () => {
  it("renders an SVG", () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector("svg")).toBeInTheDocument();
  });
  it("applies className with cn merge", () => {
    const { container } = render(<Icon name="search" className="w-8 h-8" />);
    const svg = container.querySelector("svg");
    // cn() merges conflicting Tailwind classes — w-8 should win over default w-4
    expect(svg?.className.baseVal).toContain("w-8");
    expect(svg?.className.baseVal).not.toContain("w-4 h-4 w-8"); // no duplicate
  });
  it("renders different shapes for different names", () => {
    const { container: a } = render(<Icon name="dashboard" />);
    const { container: b } = render(<Icon name="settings" />);
    expect(a.querySelector("svg")?.innerHTML).not.toBe(b.querySelector("svg")?.innerHTML);
  });
  it("sets stroke width via prop", () => {
    const { container } = render(<Icon name="search" stroke={2.5} />);
    expect(container.querySelector("svg")?.getAttribute("stroke-width")).toBe("2.5");
  });
});
