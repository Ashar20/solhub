import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { ApiClient } from "../api.js";
import { tool } from "./list_workflows.js";

function startServer(
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

describe("list_workflows tool", () => {
  let server: http.Server;
  let baseUrl: string;
  let capturedUrl: string | undefined;

  before(async () => {
    const started = await startServer((req, res) => {
      capturedUrl = req.url;
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ workflows: [] }));
    });
    server = started.server;
    baseUrl = started.url;
  });

  after(async () => {
    await stopServer(server);
  });

  it("GETs /v1/workflows?status=active&limit=10 with correct query params", async () => {
    const api = new ApiClient(baseUrl, "test-key");
    const result = await tool.handler({ status: "active", limit: 10 }, api);

    assert.equal(capturedUrl, "/v1/workflows?status=active&limit=10");
    assert.deepEqual(result, { workflows: [] });
  });

  it("GETs /v1/workflows without query params when none provided", async () => {
    const api = new ApiClient(baseUrl, "test-key");
    await tool.handler({}, api);
    assert.equal(capturedUrl, "/v1/workflows");
  });
});
