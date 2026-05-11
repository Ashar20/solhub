# SolHub

SolHub is a blockchain automation and execution reliability platform built natively for Solana. It lets users and AI agents define automation workflows (triggers, conditions, and actions) and guarantees on-chain execution via Jito bundles — no dropped transactions, no infrastructure management required. The platform integrates Geyser-backed account monitoring, Jito ShredStream for sub-slot latency, and a protocol plugin system covering Jupiter, Kamino, Marinade, Drift, Orca, Raydium, and Pyth. Workflows are registered on-chain through Anchor programs for trustlessness and marketplace discoverability, with off-chain execution handled by a Rust engine backed by PostgreSQL and a full REST API.

## Build

```bash
# Build all Rust crates
cargo build --workspace

# Install MCP server dependencies
cd mcp-server && npm install
```

## Test

```bash
# Run all Rust tests
cargo test --workspace

# Run MCP server tests (requires build first)
cd mcp-server && npm run build && npm test
```

## MCP Server

The TypeScript MCP server exposes SolHub tools for AI agent integration:

```bash
cd mcp-server
npm install
npm run build
npm start
```

## Full Specification

See [IDEA.md](./IDEA.md) for the complete backend specification including on-chain program design, engine architecture, API schema, plugin system, and database layout.
