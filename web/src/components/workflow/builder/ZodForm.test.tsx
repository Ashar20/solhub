import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { z } from "zod";
import { ZodForm } from "./ZodForm";

describe("ZodForm", () => {
  it("renders an input per string field", () => {
    const schema = z.object({ foo: z.string(), bar: z.string() });
    render(<ZodForm schema={schema} value={{ foo: "x", bar: "y" }} onChange={() => {}} />);
    expect(screen.getByDisplayValue("x")).toBeInTheDocument();
    expect(screen.getByDisplayValue("y")).toBeInTheDocument();
  });

  it("renders a select for enum field", () => {
    const schema = z.object({ side: z.enum(["long", "short"]) });
    render(<ZodForm schema={schema} value={{ side: "long" }} onChange={() => {}} />);
    expect(screen.getByDisplayValue("long")).toBeInTheDocument();
  });

  it("renders a checkbox for boolean field", () => {
    const schema = z.object({ flag: z.boolean() });
    render(<ZodForm schema={schema} value={{ flag: true }} onChange={() => {}} />);
    const cb = screen.getByRole("checkbox") as HTMLInputElement;
    expect(cb.checked).toBe(true);
  });

  it("fires onChange with merged value", () => {
    const schema = z.object({ a: z.string(), b: z.string() });
    const onChange = vi.fn();
    render(<ZodForm schema={schema} value={{ a: "1", b: "2" }} onChange={onChange} />);
    fireEvent.change(screen.getByDisplayValue("1"), { target: { value: "X" } });
    expect(onChange).toHaveBeenCalledWith({ a: "X", b: "2" });
  });

  it("unwraps ZodDefault to render an underlying number as text input", () => {
    const schema = z.object({ n: z.coerce.number().int().default(50) });
    render(<ZodForm schema={schema} value={{ n: 50 }} onChange={() => {}} />);
    expect(screen.getByDisplayValue("50")).toBeInTheDocument();
  });
});
