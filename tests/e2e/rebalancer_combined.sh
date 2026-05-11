#!/usr/bin/env bash
# Combined Signal Scout → Trade Executor E2E.
#
#   Workflow A (Signal Scout):  portfolio.snapshot → fear_greed → news → emit_webhook
#   Workflow B (Trade Executor, webhook-triggered):
#       delta_calc → jupiter.quote → guard_rails → require_approval → system.transfer
#
# We trigger Signal Scout manually (cron is unit-tested separately), it emits
# a webhook with the recommendation, that creates a Trade Executor run, the
# engine pauses at require_approval, we POST /approve, the engine resumes
# and submits a real SOL transfer on devnet.
set -uo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18087}"
DB_PATH="$REPO/solhub-combined.db"
mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
DB_URL="sqlite:$DB_PATH?mode=rwc"

cleanup() {
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

DATABASE_URL="$DB_URL" API_PORT=$API_PORT SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_TREASURY="FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb" RUST_LOG=warn ./target/release/solhub-api > "$REPO/tmp/combined-api.log" 2>&1 &
API_PID=$!
sleep 2

RAW_KEY="sk_combined_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'rebalancer-org', NULL, 10000, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'combined', NULL, $NOW, NULL);
SQL

DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/combined-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"
WEBHOOK_SECRET="rebalancer-secret"

##############################################################################
# 1. Create the TRADE EXECUTOR workflow first (webhook-triggered).
##############################################################################
# In production, params would be templated from trigger_data. For this MVP
# they're hardcoded with the same shape Signal Scout will emit.
echo "=== creating Trade Executor (workflow B) ==="
EXEC_BODY=$(cat <<JSON
{
  "name": "trade_executor",
  "trigger": {"type": "webhook", "secret": "$WEBHOOK_SECRET"},
  "steps": [
    {
      "id": "delta",
      "plugin": "solhub",
      "action": "delta_calc",
      "params": {
        "current": {"SOL": 0.60, "USDC": 0.40},
        "target":  {"SOL": 0.50, "USDC": 0.50},
        "total_value_usd": 1000.0
      }
    },
    {
      "id": "quote",
      "plugin": "jupiter",
      "action": "quote",
      "params": {
        "input_mint": "So11111111111111111111111111111111111111112",
        "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "amount": 1000000,
        "slippage_bps": 50
      }
    },
    {
      "id": "guard",
      "plugin": "solhub",
      "action": "guard_rails",
      "params": {
        "swaps": [{"from":"SOL","to":"USDC","amount_usd": 100.0}],
        "total_value_usd": 1000.0,
        "confidence_score": 0.85,
        "rules": {"max_single_swap_pct": 15.0, "max_slippage_pct": 1.0, "min_confidence": 0.6}
      }
    },
    {
      "id": "approval",
      "plugin": "solhub",
      "action": "require_approval",
      "params": {"message": "Approve SOL→USDC rebalance of 0.001 SOL?"}
    },
    {
      "id": "execute",
      "plugin": "system",
      "action": "transfer",
      "params": {"to": "11111111111111111111111111111118", "lamports": 8000}
    }
  ]
}
JSON
)
EXEC_WF=$(curl -s -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$EXEC_BODY" "$API/v1/workflows")
echo "$EXEC_WF"
EXEC_ID=$(echo "$EXEC_WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "trade_executor wf: $EXEC_ID"

##############################################################################
# 2. Create the SIGNAL SCOUT workflow (manual-trigger for demo; cron in prod).
##############################################################################
echo
echo "=== creating Signal Scout (workflow A) ==="
SCOUT_BODY=$(cat <<JSON
{
  "name": "signal_scout",
  "trigger": {"type": "manual"},
  "steps": [
    {
      "id": "snapshot",
      "plugin": "portfolio",
      "action": "snapshot",
      "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}
    },
    {
      "id": "fng",
      "plugin": "fear_greed",
      "action": "current",
      "params": {}
    },
    {
      "id": "news",
      "plugin": "news",
      "action": "fetch_headlines",
      "params": {"limit": 5}
    },
    {
      "id": "price_check",
      "plugin": "jupiter",
      "action": "price",
      "params": {"ids": ["So11111111111111111111111111111111111111112", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]}
    },
    {
      "id": "reason",
      "plugin": "llm",
      "action": "recommend_rebalance",
      "params": {
        "provider": "openai",
        "portfolio": {"current_weights": {"SOL": 0.60, "USDC": 0.40}},
        "signals": {"fng": 48, "news_summary": "neutral crypto headlines, no major events"},
        "risk_profile": "balanced"
      }
    },
    {
      "id": "emit",
      "plugin": "solhub",
      "action": "emit_webhook",
      "params": {
        "target_workflow_id": "$EXEC_ID",
        "secret": "$WEBHOOK_SECRET",
        "base_url": "$API",
        "payload": {
          "confidence": 82,
          "target_weights": {"SOL": 0.50, "USDC": 0.50},
          "reasoning": "Neutral fear/greed and stable headlines suggest moderate rebalance toward USDC.",
          "triggered_by": "manual",
          "timestamp": "2026-05-12T00:00:00Z"
        }
      }
    }
  ]
}
JSON
)
SCOUT_WF=$(curl -s -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$SCOUT_BODY" "$API/v1/workflows")
echo "$SCOUT_WF"
SCOUT_ID=$(echo "$SCOUT_WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "signal_scout wf: $SCOUT_ID"

##############################################################################
# 3. Trigger Signal Scout.
##############################################################################
echo
echo "=== triggering Signal Scout ==="
SCOUT_TRIG=$(curl -s -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$SCOUT_ID/trigger")
echo "$SCOUT_TRIG"
SCOUT_RUN=$(echo "$SCOUT_TRIG" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
echo "scout run: $SCOUT_RUN"

echo "--- waiting for Signal Scout to complete ---"
for i in $(seq 1 30); do
  STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$SCOUT_RUN" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])' 2>/dev/null)
  echo "  [$i] scout=$STATUS"
  case "$STATUS" in Confirmed|Failed|Skipped) break ;; esac
  sleep 2
done

SCOUT_DETAIL=$(curl -s -H "$AUTH" "$API/v1/runs/$SCOUT_RUN")
SCOUT_STATUS=$(echo "$SCOUT_DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
echo "Signal Scout final: $SCOUT_STATUS"
[[ "$SCOUT_STATUS" == "Confirmed" ]] || { echo "FAIL: scout didn't reach Confirmed"; exit 1; }

##############################################################################
# 4. Signal Scout's last step emitted to Trade Executor's webhook.
#    Find the Trade Executor run that was created.
##############################################################################
echo
echo "=== checking Trade Executor run was created ==="
# Wait a bit for engine to pick it up
for i in $(seq 1 20); do
  EXEC_RUNS=$(curl -s -H "$AUTH" "$API/v1/runs?workflow_id=$EXEC_ID")
  EXEC_RUN_ID=$(echo "$EXEC_RUNS" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d[0]["run_id"] if d else "")' 2>/dev/null)
  [[ -n "$EXEC_RUN_ID" ]] && break
  sleep 1
done
[[ -n "$EXEC_RUN_ID" ]] || { echo "FAIL: no Trade Executor run found"; exit 1; }
echo "executor run: $EXEC_RUN_ID"

##############################################################################
# 5. Wait for Trade Executor to reach WaitingApproval.
##############################################################################
echo "--- waiting for Trade Executor to pause at approval ---"
for i in $(seq 1 30); do
  EXEC_STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$EXEC_RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])' 2>/dev/null)
  echo "  [$i] executor=$EXEC_STATUS"
  case "$EXEC_STATUS" in WaitingApproval|Confirmed|Failed|Skipped) break ;; esac
  sleep 2
done
[[ "$EXEC_STATUS" == "WaitingApproval" ]] || { echo "FAIL: executor not at WaitingApproval (got $EXEC_STATUS)"; curl -s -H "$AUTH" "$API/v1/runs/$EXEC_RUN_ID" | python3 -m json.tool; exit 1; }
echo "  ✓ paused at approval gate"

##############################################################################
# 6. Approve.
##############################################################################
echo
echo "=== approving ==="
APPROVE_RESP=$(curl -s -w '\nHTTP=%{http_code}' -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/runs/$EXEC_RUN_ID/approve")
echo "$APPROVE_RESP"

##############################################################################
# 7. Wait for Trade Executor to complete.
##############################################################################
echo
echo "--- waiting for Trade Executor to resume + complete ---"
for i in $(seq 1 30); do
  EXEC_STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$EXEC_RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])' 2>/dev/null)
  echo "  [$i] executor=$EXEC_STATUS"
  case "$EXEC_STATUS" in Confirmed|Failed|Skipped) break ;; esac
  sleep 2
done

EXEC_DETAIL=$(curl -s -H "$AUTH" "$API/v1/runs/$EXEC_RUN_ID")
EXEC_FINAL=$(echo "$EXEC_DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
SIG=$(echo "$EXEC_DETAIL" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("signature") or "")')
echo
echo "=== FINAL ==="
echo "Signal Scout:       $SCOUT_STATUS"
echo "Trade Executor:     $EXEC_FINAL"
echo "On-chain signature: $SIG"
[[ "$EXEC_FINAL" == "Confirmed" ]] || { echo "FAIL: executor didn't reach Confirmed"; exit 1; }
[[ -n "$SIG" && "$SIG" != "null" ]] || { echo "FAIL: no signature"; exit 1; }

echo
echo "Signal Scout steps:"
echo "$SCOUT_DETAIL" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for s in d['steps_log']:
    print('  step ' + s['step_id'] + ' (' + s['status'] + ')')
"

echo "Trade Executor steps:"
echo "$EXEC_DETAIL" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for s in d['steps_log']:
    print('  step ' + s['step_id'] + ' (' + s['status'] + ')')
"

echo
echo "=== REBALANCER COMBINED E2E PASSED ==="
echo "  signal_scout_run    : $SCOUT_RUN"
echo "  trade_executor_run  : $EXEC_RUN_ID"
echo "  swap_signature      : $SIG"
echo "  explorer            : https://explorer.solana.com/tx/$SIG?cluster=devnet"
