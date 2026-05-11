import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Btn } from "./Btn";
import { Pill } from "./Pill";
import { Kbd } from "./Kbd";

describe("Btn", () => {
  it("renders children", () => {
    render(<Btn>Click</Btn>);
    expect(screen.getByRole("button", { name: "Click" })).toBeInTheDocument();
  });
  it("applies variant classes", () => {
    const { container } = render(<Btn variant="primary">Go</Btn>);
    expect(container.firstChild).toHaveClass("bg-ink-950");
  });
  it("applies size classes", () => {
    const { container } = render(<Btn size="lg">Go</Btn>);
    expect(container.firstChild).toHaveClass("h-10");
  });
  it("renders icon and children together", () => {
    render(<Btn icon={<span data-testid="ic">i</span>}>Save</Btn>);
    expect(screen.getByTestId("ic")).toBeInTheDocument();
    expect(screen.getByText("Save")).toBeInTheDocument();
  });
  it("forwards onClick", async () => {
    let clicked = false;
    const { getByRole } = render(<Btn onClick={() => { clicked = true; }}>x</Btn>);
    getByRole("button").click();
    expect(clicked).toBe(true);
  });
});

describe("Pill", () => {
  it("applies default tone", () => {
    const { container } = render(<Pill>x</Pill>);
    expect(container.firstChild).toHaveClass("bg-ink-100");
  });
  it("applies violet tone", () => {
    const { container } = render(<Pill tone="violet">v</Pill>);
    expect(container.firstChild).toHaveClass("bg-violet-50");
  });
  it("renders children", () => {
    render(<Pill>hello</Pill>);
    expect(screen.getByText("hello")).toBeInTheDocument();
  });
});

describe("Kbd", () => {
  it("renders children", () => {
    render(<Kbd>⌘K</Kbd>);
    expect(screen.getByText("⌘K")).toBeInTheDocument();
  });
});
