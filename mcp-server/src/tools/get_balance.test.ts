import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { ApiClient } from "../api.js";
import { tool } from "./get_balance.js";

function startRpcServer(
  handler: (req: http.IncomingMessage, res: http.ServerResponse) => void,
): Promise<{ server: http.Server; url: string }> {
  return new Promise((resolve) => {
    const server = http.createServer(handler);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address() as { port: number };
      resolve({ server, url: `http://127.0.0.1:${addr.port}` });
    });
  });
}

function stopServer(server: http.Server): Promise<void> {
  return new Promise((resolve, reject) => {
    server.close((err) => (err ? reject(err) : resolve()));
  });
}

describe("get_balance tool", () => {
  let rpcServer: http.Server;
  let rpcUrl: string;
  let capturedMethod: string | undefined;

  before(async () => {
    const started = await startRpcServer((req, res) => {
      let body = "";
      req.on("data", (chunk: Buffer) => { body += chunk.toString(); });
      req.on("end", () => {
        const rpcReq = JSON.parse(body) as { method: string };
        capturedMethod = rpcReq.method;

        if (rpcReq.method === "getBalance") {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            result: { value: 5_000_000_000, context: { slot: 123456 } },
          }));
        } else if (rpcReq.method === "getTokenAccountsByOwner") {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            result: {
              value: [
                {
                  pubkey: "TokenAccPubkey111",
                  account: {
                    data: {
                      parsed: {
                        info: {
                          tokenAmount: {
                            amount: "1000000",
                            decimals: 6,
                            uiAmount: 1.0,
                          },
                        },
                      },
                    },
                    lamports: 2039280,
                  },
                },
              ],
            },
          }));
        } else {
          res.writeHead(400);
          res.end("unknown method");
        }
      });
    });
    rpcServer = started.server;
    rpcUrl = started.url;
  });

  after(async () => {
    await stopServer(rpcServer);
  });

  it("queries Solana RPC getBalance for SOL balance", async () => {
    // Point the tool at our fake RPC by setting env var
    const originalRpc = process.env.SOLANA_RPC_URL;
    process.env.SOLANA_RPC_URL = rpcUrl;

    try {
      const api = new ApiClient("http://localhost:8080", "key");
      const result = await tool.handler(
        { account: "So11111111111111111111111111111111111111112" },
        api,
      );

      assert.equal(capturedMethod, "getBalance");
      const r = result as { lamports: number; sol: number; slot: number };
      assert.equal(r.lamports, 5_000_000_000);
      assert.equal(r.sol, 5);
      assert.equal(r.slot, 123456);
    } finally {
      if (originalRpc === undefined) {
        delete process.env.SOLANA_RPC_URL;
      } else {
        process.env.SOLANA_RPC_URL = originalRpc;
      }
    }
  });

  it("queries Solana RPC getTokenAccountsByOwner for SPL balance", async () => {
    const originalRpc = process.env.SOLANA_RPC_URL;
    process.env.SOLANA_RPC_URL = rpcUrl;

    try {
      const api = new ApiClient("http://localhost:8080", "key");
      const result = await tool.handler(
        {
          account: "So11111111111111111111111111111111111111112",
          token_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        },
        api,
      );

      assert.equal(capturedMethod, "getTokenAccountsByOwner");
      const r = result as { amount: string; ui_amount: number; decimals: number };
      assert.equal(r.amount, "1000000");
      assert.equal(r.ui_amount, 1.0);
      assert.equal(r.decimals, 6);
    } finally {
      if (originalRpc === undefined) {
        delete process.env.SOLANA_RPC_URL;
      } else {
        process.env.SOLANA_RPC_URL = originalRpc;
      }
    }
  });
});
