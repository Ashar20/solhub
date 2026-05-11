#!/usr/bin/env bash
# Per-node smoke tests for every new Signal Scout / Trade Executor plugin action.
set -uo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18086}"
DB_PATH="$REPO/solhub-nodes.db"
mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
DB_URL="sqlite:$DB_PATH?mode=rwc"

cleanup() {
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

DATABASE_URL="$DB_URL" API_PORT=$API_PORT SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_TREASURY="FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb" RUST_LOG=warn ./target/release/solhub-api > "$REPO/tmp/nodes-api.log" 2>&1 &
API_PID=$!
sleep 2

RAW_KEY="sk_nodes_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'nodes-org', NULL, 10000, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'nodes', NULL, $NOW, NULL);
SQL

DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/nodes-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"
PASS=0; FAIL=0
RESULTS=()
COUNTER=0

run_node() {
  local name="$1" steps="$2" expect="$3"
  COUNTER=$((COUNTER+1))
  echo "----- [$COUNTER] $name -----"
  local sanitized="node_${COUNTER}"
  local BODY="{\"name\":\"$sanitized\",\"trigger\":{\"type\":\"manual\"},\"steps\":$steps}"
  local WF=$(curl -s -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$BODY" "$API/v1/workflows")
  if ! echo "$WF" | python3 -c 'import sys,json;json.load(sys.stdin)["workflow_id"]' >/dev/null 2>&1; then
    echo "  CREATE FAILED: $WF"
    FAIL=$((FAIL+1)); RESULTS+=("FAIL $name (create: $WF)")
    return
  fi
  local WF_ID=$(echo "$WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
  local TRIG=$(curl -s -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$WF_ID/trigger")
  if ! echo "$TRIG" | python3 -c 'import sys,json;json.load(sys.stdin)["run_id"]' >/dev/null 2>&1; then
    echo "  TRIGGER FAILED: $TRIG"
    FAIL=$((FAIL+1)); RESULTS+=("FAIL $name (trigger: $TRIG)")
    return
  fi
  local RUN_ID=$(echo "$TRIG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
  for i in $(seq 1 30); do
    local STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("status",""))' 2>/dev/null)
    case "$STATUS" in
      Confirmed|Failed|Skipped|WaitingApproval) break ;;
    esac
    sleep 1
  done
  local DETAIL=$(curl -s -H "$AUTH" "$API/v1/runs/$RUN_ID")
  local FINAL=$(echo "$DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  local OUT_PREVIEW=$(echo "$DETAIL" | python3 -c 'import sys,json;d=json.load(sys.stdin); s=d["steps_log"];print(json.dumps(s[0].get("output") if s else None))[:180]' 2>/dev/null)
  local ERR=$(echo "$DETAIL" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("error_message",""))' 2>/dev/null)
  echo "  status=$FINAL"
  echo "  output=$OUT_PREVIEW"
  [[ -n "$ERR" ]] && echo "  error=$ERR"
  if [[ "$FINAL" == "$expect" ]]; then
    PASS=$((PASS+1)); RESULTS+=("PASS $name ($FINAL)")
  else
    FAIL=$((FAIL+1)); RESULTS+=("FAIL $name (got $FINAL, expected $expect)")
  fi
}

echo "========= INDIVIDUAL NODE TESTS ========="

run_node "portfolio.snapshot" \
  '[{"id":"s","plugin":"portfolio","action":"snapshot","params":{"account":"FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}]' \
  "Confirmed"

run_node "portfolio.detect_drift" \
  '[{"id":"s","plugin":"portfolio","action":"detect_drift","params":{"current_weights":{"SOL":0.6,"USDC":0.4},"target_weights":{"SOL":0.5,"USDC":0.5},"threshold":0.05}}]' \
  "Confirmed"

run_node "jupiter.price" \
  '[{"id":"s","plugin":"jupiter","action":"price","params":{"ids":["So11111111111111111111111111111111111111112"]}}]' \
  "Confirmed"

run_node "jupiter.quote" \
  '[{"id":"s","plugin":"jupiter","action":"quote","params":{"input_mint":"So11111111111111111111111111111111111111112","output_mint":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","amount":1000000,"slippage_bps":50}}]' \
  "Confirmed"

run_node "fear_greed.current" \
  '[{"id":"s","plugin":"fear_greed","action":"current","params":{}}]' \
  "Confirmed"

run_node "news.fetch_headlines" \
  '[{"id":"s","plugin":"news","action":"fetch_headlines","params":{"limit":3}}]' \
  "Confirmed"

# crypto_panic needs an API token; expect Failed without one
if [[ -n "${CRYPTOPANIC_TOKEN:-}" ]]; then EXP_CP="Confirmed"; else EXP_CP="Failed"; fi
run_node "news.crypto_panic" \
  '[{"id":"s","plugin":"news","action":"crypto_panic","params":{"limit":3}}]' \
  "$EXP_CP"

# solhub.delta_calc — using actual implemented param names: current, target
run_node "solhub.delta_calc" \
  '[{"id":"s","plugin":"solhub","action":"delta_calc","params":{"current":{"SOL":0.6,"USDC":0.4},"target":{"SOL":0.4,"USDC":0.6},"total_value_usd":1000.0}}]' \
  "Confirmed"

# guard_rails: uses confidence_score on 0-1 scale (min_confidence default 0.6)
run_node "solhub.guard_rails_pass" \
  '[{"id":"s","plugin":"solhub","action":"guard_rails","params":{"swaps":[{"from":"SOL","to":"USDC","amount_usd":100.0}],"total_value_usd":1000.0,"confidence_score":0.85,"quotes":[{"priceImpactPct":"0.3"}]}}]' \
  "Confirmed"

run_node "solhub.guard_rails_block" \
  '[{"id":"s","plugin":"solhub","action":"guard_rails","params":{"swaps":[{"from":"SOL","to":"USDC","amount_usd":500.0}],"total_value_usd":1000.0,"confidence_score":0.85,"quotes":[{"priceImpactPct":"0.3"}]}}]' \
  "Confirmed"

run_node "solhub.require_approval" \
  '[{"id":"s","plugin":"solhub","action":"require_approval","params":{"message":"go?"}}]' \
  "WaitingApproval"

# llm.recommend_rebalance — without API key, expected to Fail
if [[ -n "${OPENAI_API_KEY:-}" || -n "${ANTHROPIC_API_KEY:-}" ]]; then EXP_LLM="Confirmed"; else EXP_LLM="Failed"; fi
run_node "llm.recommend_rebalance" \
  '[{"id":"s","plugin":"llm","action":"recommend_rebalance","params":{"portfolio":{"current_weights":{"SOL":0.6,"USDC":0.4}},"signals":{"fng":65},"risk_profile":"balanced"}}]' \
  "$EXP_LLM"

# solhub.emit_webhook — first create a dummy webhook-triggered workflow as the target
WEBHOOK_BODY='{"name":"webhook_target","trigger":{"type":"webhook","secret":"sekret"},"steps":[{"id":"s","plugin":"system","action":"get_balance","params":{"account":"FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}]}'
WEBHOOK_WF=$(curl -s -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$WEBHOOK_BODY" "$API/v1/workflows")
WEBHOOK_WF_ID=$(echo "$WEBHOOK_WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "webhook target wf: $WEBHOOK_WF_ID"

run_node "solhub.emit_webhook" \
  "[{\"id\":\"s\",\"plugin\":\"solhub\",\"action\":\"emit_webhook\",\"params\":{\"target_workflow_id\":\"$WEBHOOK_WF_ID\",\"payload\":{\"hello\":\"world\"},\"secret\":\"sekret\",\"base_url\":\"$API\"}}]" \
  "Confirmed"

echo
echo "========= NODE TEST SUMMARY ========="
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo "  total: $((PASS+FAIL)) | pass=$PASS fail=$FAIL"
[[ $FAIL -eq 0 ]] && echo "ALL NODES PASS" || echo "SOME NODES FAILED"
exit $FAIL
