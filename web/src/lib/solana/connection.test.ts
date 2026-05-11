import { describe, it, expect } from "vitest";
import { connection, NETWORK } from "./connection";

describe("connection", () => {
  it("creates a Connection object", () => {
    expect(connection).toBeTruthy();
    expect(typeof connection.getBalance).toBe("function");
  });
  it("exports a network string", () => {
    expect(["mainnet", "devnet"]).toContain(NETWORK);
  });
});
