import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { SolhubLogo } from "./SolhubLogo";
import { SolanaMark } from "./SolanaMark";

describe("SolanaMark", () => {
  it("renders SVG with gradient", () => {
    const { container } = render(<SolanaMark />);
    expect(container.querySelector("svg")).toBeInTheDocument();
    expect(container.querySelector("linearGradient")).toBeInTheDocument();
  });
});

describe("SolhubLogo", () => {
  it("renders the brand mark and label", () => {
    const { container } = render(<SolhubLogo />);
    expect(container.textContent).toContain("solhub");
    expect(container.textContent).toContain("workflow os");
  });
});
