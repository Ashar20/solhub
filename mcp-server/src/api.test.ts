import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { ApiClient } from "./api.js";

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

describe("ApiClient", () => {
  let server: http.Server;
  let baseUrl: string;
  let capturedAuthHeader: string | undefined;

  before(async () => {
    const started = await startServer((req, res) => {
      capturedAuthHeader = req.headers["authorization"];
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ ok: true }));
    });
    server = started.server;
    baseUrl = started.url;
  });

  after(async () => {
    await stopServer(server);
  });

  it("sets Bearer auth header when apiKey is provided", async () => {
    const client = new ApiClient(baseUrl, "test-key-123");
    await client.get("/some-path");
    assert.equal(capturedAuthHeader, "Bearer test-key-123");
  });

  it("throws on non-2xx responses", async () => {
    const errorServer = await startServer((_req, res) => {
      res.writeHead(404, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: "not found" }));
    });

    try {
      const client = new ApiClient(errorServer.url, "key");
      await assert.rejects(
        () => client.get("/missing"),
        (err: Error) => {
          assert.match(err.message, /SolHub API 404/);
          return true;
        },
      );
    } finally {
      await stopServer(errorServer.server);
    }
  });
});
