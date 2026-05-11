import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ToolPalette } from "./ToolPalette";

describe("ToolPalette", () => {
  it("renders plugin groups + actions", () => {
    render(<ToolPalette onAdd={() => {}} />);
    // From registry — Jupiter is definitely there
    expect(screen.getByText("Jupiter")).toBeInTheDocument();
    expect(screen.getByText("Swap Tokens")).toBeInTheDocument();
  });

  it("filters by search query", () => {
    render(<ToolPalette onAdd={() => {}} />);
    fireEvent.change(screen.getByPlaceholderText("Search tools…"), { target: { value: "jupiter" } });
    expect(screen.getByText("Jupiter")).toBeInTheDocument();
    // Pyth should not appear in the filtered list
    expect(screen.queryByText("Pyth")).not.toBeInTheDocument();
  });

  it("shows empty state when no match", () => {
    render(<ToolPalette onAdd={() => {}} />);
    fireEvent.change(screen.getByPlaceholderText("Search tools…"), { target: { value: "zzzz-no-match-zzzz" } });
    expect(screen.getByText("No tools match.")).toBeInTheDocument();
  });

  it("fires onAdd with plugin+action ids", () => {
    const onAdd = vi.fn();
    render(<ToolPalette onAdd={onAdd} />);
    // The button's aria-label is `Add ${plugin.name} ${action.name}`
    fireEvent.click(screen.getByLabelText(/Add Jupiter Swap Tokens/i));
    expect(onAdd).toHaveBeenCalledWith("jupiter", "swap");
  });
});
