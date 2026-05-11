#!/usr/bin/env bash
# Supplemental E2E: MCP server tools + SSE log streaming against a running API.
# Re-uses the devnet keypair + spins API+engine briefly.

set -euo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18081}"
DB_PATH="$REPO/solhub-e2e-mcp.db"
API_LOG="$REPO/tmp/e2e-api-mcp.log"
ENGINE_LOG="$REPO/tmp/e2e-engine-mcp.log"

mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"

cleanup() {
  echo "--- cleanup ---"
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

DB_URL="sqlite:$DB_PATH?mode=rwc"
echo "--- start API ---"
DATABASE_URL="$DB_URL" API_PORT=$API_PORT RUST_LOG=warn ./target/release/solhub-api > "$API_LOG" 2>&1 &
API_PID=$!
sleep 2
curl -sf "http://localhost:$API_PORT/health" > /dev/null

# Seed org + api key
RAW_KEY="sk_mcp_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at)
VALUES ('$ORG_ID', 'mcp-org', NULL, 0, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at)
VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'mcp', NULL, $NOW, NULL);
SQL

echo "--- start engine ---"
DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$ENGINE_LOG" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"

# ── MCP smoke: tools/list ────────────────────────────────────────────────
echo "--- MCP tools/list ---"
MCP_OUT=$(
  ( printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e","version":"0.1"}}}'
    sleep 0.2
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    sleep 0.2
    printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
    sleep 1
  ) | SOLHUB_API_URL="$API" SOLHUB_API_KEY="$RAW_KEY" node mcp-server/dist/index.js 2>/dev/null
)
echo "$MCP_OUT" | head -200
TOOL_COUNT=$(echo "$MCP_OUT" | python3 -c '
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line: continue
    try:
        d = json.loads(line)
    except: continue
    if d.get("id") == 2:
        print(len(d.get("result", {}).get("tools", [])))
        break
')
echo "MCP tool count: $TOOL_COUNT"
[[ "$TOOL_COUNT" == "7" ]] || { echo "FAIL: expected 7 MCP tools"; exit 1; }

# ── MCP tool call: sk.list_workflows ──────────────────────────────────────
echo "--- MCP tools/call sk.list_workflows ---"
MCP_CALL=$(
  ( printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e","version":"0.1"}}}'
    sleep 0.2
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    sleep 0.2
    printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"sk.list_workflows","arguments":{"limit":5}}}'
    sleep 1
  ) | SOLHUB_API_URL="$API" SOLHUB_API_KEY="$RAW_KEY" node mcp-server/dist/index.js 2>/dev/null
)
echo "$MCP_CALL" | tail -3
echo "$MCP_CALL" | grep -q '"id":3' || { echo "FAIL: no response to id=3"; exit 1; }

# ── SSE: create a workflow, trigger it, stream logs ─────────────────────
echo "--- create workflow for SSE ---"
WF=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{
  "name": "sse-test",
  "trigger": {"type": "manual"},
  "steps": [{"id": "s1", "plugin": "system", "action": "get_balance", "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}]
}' "$API/v1/workflows")
WF_ID=$(echo "$WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "wf=$WF_ID"

TRIG=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$WF_ID/trigger")
RUN_ID=$(echo "$TRIG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
echo "run=$RUN_ID"

echo "--- SSE stream (15s max) ---"
SSE_OUT=$(timeout 15 curl -sf -N -H "$AUTH" -H "Accept: text/event-stream" "$API/v1/runs/$RUN_ID/logs" || true)
echo "$SSE_OUT" | head -30
echo "$SSE_OUT" | grep -q 'event: run_complete' || { echo "FAIL: no run_complete event"; exit 1; }
echo "SSE: run_complete event received"

echo
echo "=== MCP + SSE PASSED ==="
echo "  MCP tools/list returned 7 tools"
echo "  MCP tools/call sk.list_workflows answered id=3"
echo "  SSE delivered run_complete for run_id=$RUN_ID"
