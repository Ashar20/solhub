#!/usr/bin/env bash
# Sub-workflow E2E: a parent workflow triggers a child workflow and depends on
# the child's result via the solhub.run_workflow plugin action.
#
# Parent steps:
#   1. solhub.run_workflow → child workflow (which does a SOL transfer)
#   2. system.get_balance  → read final balance
#
# Verifies: parent's run reaches Confirmed, step_log[0].output contains the
# child run's signature, both runs exist in DB.
set -euo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18084}"
DB_PATH="$REPO/solhub-sub.db"

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
DATABASE_URL="$DB_URL" \
  API_PORT=$API_PORT \
  SOLANA_RPC_URL="https://api.devnet.solana.com" \
  SOLHUB_TREASURY="FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb" \
  RUST_LOG=warn \
  ./target/release/solhub-api > "$REPO/tmp/sub-api.log" 2>&1 &
API_PID=$!
sleep 2

RAW_KEY="sk_sub_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'sub-org', NULL, 0, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'sub', NULL, $NOW, NULL);
SQL

echo "--- start engine ---"
DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/sub-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"

echo "--- create CHILD workflow (transfer 3000 lamports) ---"
CHILD_BODY='{
  "name": "child-transfer",
  "trigger": {"type": "manual"},
  "steps": [{
    "id": "do_transfer",
    "plugin": "system",
    "action": "transfer",
    "params": {"to": "11111111111111111111111111111114", "lamports": 3000}
  }]
}'
CHILD=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$CHILD_BODY" "$API/v1/workflows")
CHILD_ID=$(echo "$CHILD" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "  child_id=$CHILD_ID"

echo "--- create PARENT workflow that calls the child ---"
PARENT_BODY=$(cat <<JSON
{
  "name": "parent-with-subworkflow",
  "trigger": {"type": "manual"},
  "steps": [
    {
      "id": "call_child",
      "plugin": "solhub",
      "action": "run_workflow",
      "params": {"workflow_id": "$CHILD_ID", "timeout_secs": 30}
    },
    {
      "id": "check_balance",
      "plugin": "system",
      "action": "get_balance",
      "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}
    }
  ]
}
JSON
)
PARENT=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$PARENT_BODY" "$API/v1/workflows")
PARENT_ID=$(echo "$PARENT" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "  parent_id=$PARENT_ID"

echo "--- trigger PARENT ---"
TRIG=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$PARENT_ID/trigger")
PARENT_RUN=$(echo "$TRIG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
echo "  parent_run=$PARENT_RUN"

echo "--- poll PARENT run status ---"
for i in $(seq 1 60); do
  STATUS=$(curl -sf -H "$AUTH" "$API/v1/runs/$PARENT_RUN" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  echo "  [$i] $STATUS"
  case "$STATUS" in Confirmed|Failed|Skipped) break ;; esac
  sleep 2
done

DETAIL=$(curl -sf -H "$AUTH" "$API/v1/runs/$PARENT_RUN")
echo "--- PARENT run detail ---"
echo "$DETAIL" | python3 -m json.tool

# Extract child run id from parent's step output
CHILD_RUN_ID=$(echo "$DETAIL" | python3 -c '
import sys, json
d = json.load(sys.stdin)
steps = d["steps_log"]
for s in steps:
    if s.get("step_id") == "call_child":
        out = s.get("output", {})
        print(out.get("child_run_id", ""))
        break
')
echo "--- child_run_id=$CHILD_RUN_ID ---"

CHILD_DETAIL=$(curl -sf -H "$AUTH" "$API/v1/runs/$CHILD_RUN_ID")
echo "--- CHILD run detail ---"
echo "$CHILD_DETAIL" | python3 -m json.tool

# Assertions
PARENT_STATUS=$(echo "$DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
[[ "$PARENT_STATUS" == "Confirmed" ]] || { echo "FAIL: parent not Confirmed ($PARENT_STATUS)"; exit 1; }

CHILD_STATUS=$(echo "$CHILD_DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
[[ "$CHILD_STATUS" == "Confirmed" ]] || { echo "FAIL: child not Confirmed ($CHILD_STATUS)"; exit 1; }

CHILD_SIG=$(echo "$CHILD_DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["signature"] or "")')
[[ -n "$CHILD_SIG" && "$CHILD_SIG" != "null" ]] || { echo "FAIL: child has no signature"; exit 1; }

CHILD_TRIGGERED_BY=$(echo "$CHILD_DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["triggered_by"])')
[[ "$CHILD_TRIGGERED_BY" == parent:* ]] || { echo "FAIL: child triggered_by should be 'parent:...' got $CHILD_TRIGGERED_BY"; exit 1; }

echo
echo "=== SUB-WORKFLOW E2E PASSED ==="
echo "  parent_run    : $PARENT_RUN ($PARENT_STATUS)"
echo "  child_run     : $CHILD_RUN_ID ($CHILD_STATUS)"
echo "  child_signature: $CHILD_SIG"
echo "  child_triggered_by: $CHILD_TRIGGERED_BY"
echo "  explorer       : https://explorer.solana.com/tx/$CHILD_SIG?cluster=devnet"
