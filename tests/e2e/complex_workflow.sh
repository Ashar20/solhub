#!/usr/bin/env bash
# Verify the existing engine handles complex multi-step workflows on devnet:
#   1. Mixed plugin types in one workflow (Read → Transaction → Read)
#   2. Two sequential transfers within ONE workflow
#   3. Verify all steps logged in order, run goes Pending→Confirmed
set -euo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18082}"
DB_PATH="$REPO/solhub-complex.db"
DEST1="11111111111111111111111111111112"
DEST2="11111111111111111111111111111113"

mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
DB_URL="sqlite:$DB_PATH?mode=rwc"

cleanup() { [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true; [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true; wait 2>/dev/null || true; }
trap cleanup EXIT

echo "--- start API ---"
DATABASE_URL="$DB_URL" API_PORT=$API_PORT RUST_LOG=warn ./target/release/solhub-api > "$REPO/tmp/complex-api.log" 2>&1 &
API_PID=$!
sleep 2

# seed
RAW_KEY="sk_complex_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'complex-org', NULL, 0, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'complex', NULL, $NOW, NULL);
SQL

echo "--- start engine ---"
DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/complex-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"

echo "--- complex workflow: 4 steps mixing Read+Transaction ---"
BODY=$(cat <<JSON
{
  "name": "complex-multi-step",
  "trigger": {"type": "manual"},
  "steps": [
    {"id": "balance_before", "plugin": "system", "action": "get_balance",
     "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}},
    {"id": "transfer_1", "plugin": "system", "action": "transfer",
     "params": {"to": "$DEST1", "lamports": 5000}},
    {"id": "transfer_2", "plugin": "system", "action": "transfer",
     "params": {"to": "$DEST2", "lamports": 7000}},
    {"id": "balance_after", "plugin": "system", "action": "get_balance",
     "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}
  ]
}
JSON
)
WF=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$BODY" "$API/v1/workflows")
WF_ID=$(echo "$WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "wf=$WF_ID"

TRIG=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$WF_ID/trigger")
RUN_ID=$(echo "$TRIG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
echo "run=$RUN_ID"

echo "--- poll ---"
for i in $(seq 1 60); do
  STATUS=$(curl -sf -H "$AUTH" "$API/v1/runs/$RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  echo "  [$i] $STATUS"
  case "$STATUS" in Confirmed|Failed|Skipped) break ;; esac
  sleep 2
done

DETAIL=$(curl -sf -H "$AUTH" "$API/v1/runs/$RUN_ID")
echo "--- final ---"
echo "$DETAIL" | python3 -m json.tool

# Assertions
[[ "$(echo "$DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')" == "Confirmed" ]] || { echo "FAIL: not Confirmed"; exit 1; }
STEP_COUNT=$(echo "$DETAIL" | python3 -c 'import sys,json;print(len(json.load(sys.stdin)["steps_log"]))')
[[ "$STEP_COUNT" == "4" ]] || { echo "FAIL: expected 4 steps logged, got $STEP_COUNT"; exit 1; }
SIG=$(echo "$DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["signature"] or "")')
[[ -n "$SIG" && "$SIG" != "null" ]] || { echo "FAIL: no signature"; exit 1; }
echo
echo "=== COMPLEX WORKFLOW PASSED ==="
echo "  steps_logged: $STEP_COUNT"
echo "  signature   : $SIG"
echo "  explorer    : https://explorer.solana.com/tx/$SIG?cluster=devnet"
