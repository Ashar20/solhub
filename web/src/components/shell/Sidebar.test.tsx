import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { Sidebar } from "./Sidebar";

vi.mock("next/navigation", () => ({ usePathname: () => "/workflows" }));

describe("Sidebar", () => {
  it("renders all 8 nav items", () => {
    render(<Sidebar />);
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Workflows")).toBeInTheDocument();
    expect(screen.getByText("AI Builder")).toBeInTheDocument();
    expect(screen.getByText("Runs & Logs")).toBeInTheDocument();
    expect(screen.getByText("Marketplace")).toBeInTheDocument();
    expect(screen.getByText("Wallet")).toBeInTheDocument();
    expect(screen.getByText("Versions")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });
  it("highlights the active route", () => {
    render(<Sidebar />);
    const link = screen.getByText("Workflows").closest("a");
    expect(link?.className).toContain("bg-ink-100");
  });
  it("does not highlight inactive routes", () => {
    render(<Sidebar />);
    const link = screen.getByText("Marketplace").closest("a");
    expect(link?.className).not.toContain("bg-ink-100");
  });
});
