#!/usr/bin/env bash
# Batch transfer E2E: a single workflow step using system.batch_transfer
# moves SOL to 3 recipients in ONE on-chain transaction.
set -euo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18085}"
DB_PATH="$REPO/solhub-batch.db"

mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
DB_URL="sqlite:$DB_PATH?mode=rwc"

cleanup() {
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

echo "--- start API ---"
DATABASE_URL="$DB_URL" API_PORT=$API_PORT SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_TREASURY="FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb" RUST_LOG=warn ./target/release/solhub-api > "$REPO/tmp/batch-api.log" 2>&1 &
API_PID=$!
sleep 2

RAW_KEY="sk_batch_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'batch-org', NULL, 0, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'batch', NULL, $NOW, NULL);
SQL

echo "--- start engine ---"
DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/batch-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"

echo "--- create batch_transfer workflow (3 recipients in one tx) ---"
BODY='{
  "name": "batch-3",
  "trigger": {"type": "manual"},
  "steps": [{
    "id": "batch",
    "plugin": "system",
    "action": "batch_transfer",
    "params": {
      "transfers": [
        {"to": "11111111111111111111111111111115", "lamports": 1000},
        {"to": "11111111111111111111111111111116", "lamports": 2000},
        {"to": "11111111111111111111111111111117", "lamports": 3000}
      ]
    }
  }]
}'
WF=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$BODY" "$API/v1/workflows")
WF_ID=$(echo "$WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')

TRIG=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$WF_ID/trigger")
RUN_ID=$(echo "$TRIG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
echo "run=$RUN_ID"

for i in $(seq 1 60); do
  STATUS=$(curl -sf -H "$AUTH" "$API/v1/runs/$RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  echo "  [$i] $STATUS"
  case "$STATUS" in Confirmed|Failed|Skipped) break ;; esac
  sleep 2
done

DETAIL=$(curl -sf -H "$AUTH" "$API/v1/runs/$RUN_ID")
echo "$DETAIL" | python3 -m json.tool
SIG=$(echo "$DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["signature"] or "")')
[[ "$(echo "$DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')" == "Confirmed" ]] || { echo "FAIL"; exit 1; }
[[ -n "$SIG" && "$SIG" != "null" ]] || { echo "FAIL: no signature"; exit 1; }

# Verify ON-CHAIN: signature should be a single tx with 3 transfer ixs
PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH" solana confirm -v "$SIG" --url devnet 2>&1 | head -30

echo
echo "=== BATCH_TRANSFER E2E PASSED ==="
echo "  signature: $SIG"
echo "  explorer : https://explorer.solana.com/tx/$SIG?cluster=devnet"
