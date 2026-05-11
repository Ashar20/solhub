#!/usr/bin/env node
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

import { ApiClient } from "./api.js";
import { tool as createWorkflow } from "./tools/create_workflow.js";
import { tool as triggerWorkflow } from "./tools/trigger_workflow.js";
import { tool as getRunStatus } from "./tools/get_run_status.js";
import { tool as listWorkflows } from "./tools/list_workflows.js";
import { tool as getBalance } from "./tools/get_balance.js";
import { tool as callProgram } from "./tools/call_program.js";
import { tool as publishToHub } from "./tools/publish_to_hub.js";

const tools = [
  createWorkflow,
  triggerWorkflow,
  getRunStatus,
  listWorkflows,
  getBalance,
  callProgram,
  publishToHub,
];

const toolMap = new Map(tools.map((t) => [t.name, t]));

const server = new Server(
  { name: "solhub", version: "0.1.0" },
  { capabilities: { tools: {} } },
);

const api = new ApiClient();

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: tools.map((t) => ({
    name: t.name,
    description: t.description,
    inputSchema: t.inputSchema,
  })),
}));

server.setRequestHandler(CallToolRequestSchema, async (req) => {
  const tool = toolMap.get(req.params.name);
  if (!tool) throw new Error(`unknown tool: ${req.params.name}`);
  const result = await tool.handler(req.params.arguments ?? {}, api);
  return {
    content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
  };
});

const transport = new StdioServerTransport();
await server.connect(transport);
