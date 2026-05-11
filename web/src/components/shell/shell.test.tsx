import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Breadcrumb } from "./Breadcrumb";
import { Topbar } from "./Topbar";

describe("Breadcrumb", () => {
  it("renders each item", () => {
    render(<Breadcrumb items={["Workspace", "solhub-prod", "Dashboard"]} />);
    expect(screen.getByText("Workspace")).toBeInTheDocument();
    expect(screen.getByText("solhub-prod")).toBeInTheDocument();
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
  });
  it("marks the last item with emphasis styling", () => {
    render(<Breadcrumb items={["A", "B", "C"]} />);
    expect(screen.getByText("C").className).toContain("text-ink-900");
    expect(screen.getByText("A").className).not.toContain("text-ink-900");
  });
});

describe("Topbar", () => {
  it("renders breadcrumb + search input", () => {
    render(<Topbar crumbs={["Workspace", "Dashboard"]} />);
    expect(screen.getByText("Workspace")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Search workspace…")).toBeInTheDocument();
  });
  it("renders right slot content", () => {
    render(<Topbar crumbs={["X"]} right={<button data-testid="r">R</button>} />);
    expect(screen.getByTestId("r")).toBeInTheDocument();
  });
});
