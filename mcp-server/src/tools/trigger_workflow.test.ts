import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { ApiClient } from "../api.js";
import { tool } from "./trigger_workflow.js";

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

describe("trigger_workflow tool", () => {
  let server: http.Server;
  let baseUrl: string;
  let capturedMethod: string | undefined;
  let capturedPath: string | undefined;

  before(async () => {
    const started = await startServer((req, res) => {
      capturedMethod = req.method;
      capturedPath = req.url;
      // consume body
      req.resume();
      req.on("end", () => {
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ run_id: "run-456", status: "triggered" }));
      });
    });
    server = started.server;
    baseUrl = started.url;
  });

  after(async () => {
    await stopServer(server);
  });

  it("POSTs to /v1/workflows/:id/trigger", async () => {
    const api = new ApiClient(baseUrl, "test-key");
    const result = await tool.handler({ workflow_id: "wf-abc" }, api);

    assert.equal(capturedMethod, "POST");
    assert.equal(capturedPath, "/v1/workflows/wf-abc/trigger");
    assert.deepEqual(result, { run_id: "run-456", status: "triggered" });
  });
});
