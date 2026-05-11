import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { ApiClient } from "../api.js";
import { tool } from "./create_workflow.js";

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

describe("create_workflow tool", () => {
  let server: http.Server;
  let baseUrl: string;
  let capturedMethod: string | undefined;
  let capturedPath: string | undefined;
  let capturedBody: unknown;

  before(async () => {
    const started = await startServer((req, res) => {
      capturedMethod = req.method;
      capturedPath = req.url;
      let body = "";
      req.on("data", (chunk: Buffer) => { body += chunk.toString(); });
      req.on("end", () => {
        capturedBody = JSON.parse(body);
        res.writeHead(201, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ workflow_id: "wf-123", status: "created" }));
      });
    });
    server = started.server;
    baseUrl = started.url;
  });

  after(async () => {
    await stopServer(server);
  });

  it("POSTs to /v1/workflows and returns the response", async () => {
    const api = new ApiClient(baseUrl, "test-key");
    const result = await tool.handler(
      {
        name: "my-workflow",
        trigger: { type: "cron", schedule: "*/5 * * * *" },
        steps: [{ plugin: "jupiter", action: "swap", params: { amount: 100 } }],
      },
      api,
    );

    assert.equal(capturedMethod, "POST");
    assert.equal(capturedPath, "/v1/workflows");
    assert.deepEqual(result, { workflow_id: "wf-123", status: "created" });

    const body = capturedBody as { name: string; trigger: { type: string } };
    assert.equal(body.name, "my-workflow");
    assert.equal(body.trigger.type, "cron");
  });

  it("rejects invalid input (missing required fields)", async () => {
    const api = new ApiClient(baseUrl, "test-key");
    await assert.rejects(
      () => tool.handler({ name: "bad" }, api),
      (err: Error) => {
        assert.ok(err.message.includes("trigger") || err.message.includes("steps") || err.name === "ZodError");
        return true;
      },
    );
  });
});
