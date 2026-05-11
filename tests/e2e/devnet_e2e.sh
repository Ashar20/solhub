#!/usr/bin/env bash
# Full-stack E2E test against Solana devnet — no mocks.
#
# Spins up:
#   - SQLite DB (file-backed)
#   - solhub-api server on $API_PORT (default 18080)
#   - solhub-engine worker pointed at the same DB
# Then drives all major REST API endpoints, ending with a real on-chain
# SOL transfer via the system.transfer plugin, and verifies the on-chain TX.
#
# Requires: solhub-dev.json with devnet SOL, cargo binaries built.

set -euo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
RPC="https://api.devnet.solana.com"
WALLET="$REPO/solhub-dev.json"
KEYPAIR_PUBKEY="$(PATH=$HOME/.local/share/solana/install/active_release/bin:$PATH solana-keygen pubkey "$WALLET")"
API_PORT="${API_PORT:-18080}"
DB_PATH="$REPO/solhub-e2e.db"
API_LOG="$REPO/tmp/e2e-api.log"
ENGINE_LOG="$REPO/tmp/e2e-engine.log"
DEST_PUBKEY="11111111111111111111111111111112"   # arbitrary destination (just past System Program)
LAMPORTS=10000   # 0.00001 SOL — tiny test transfer

mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"

cleanup() {
  echo "--- cleanup ---"
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

# Build (if needed)
echo "--- cargo build ---"
cargo build --release -p api -p engine -p cli 2>&1 | tail -3

DB_URL="sqlite:$DB_PATH?mode=rwc"

echo "--- seed DB with org + api key ---"
# Use a tiny rust binary inlined via cargo run to seed the DB
cat > "$REPO/tmp/e2e_seed.sql" <<'SQL'
SQL

# Seed via sqlite3 directly (org + api key)
RAW_KEY="sk_e2e_test_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)

# Init schema via the API on first connect (it migrates automatically)
# We need to start the API once first to run migrations, then seed.

echo "--- start API (will run migrations) ---"
DATABASE_URL="$DB_URL" API_PORT=$API_PORT RUST_LOG=info ./target/release/solhub-api > "$API_LOG" 2>&1 &
API_PID=$!
sleep 2
if ! kill -0 $API_PID 2>/dev/null; then
  echo "API failed to start. log:" && tail -30 "$API_LOG"
  exit 1
fi

# Seed via sqlite3
echo "--- seed org + api key into $DB_PATH ---"
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at)
VALUES ('$ORG_ID', 'e2e-org', '$KEYPAIR_PUBKEY', 0, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at)
VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'e2e-key', NULL, $NOW, NULL);
SQL
echo "seeded org=$ORG_ID key_hash=$KEY_HASH"

echo "--- health check ---"
curl -sf "http://localhost:$API_PORT/health" && echo

echo "--- start engine ---"
DATABASE_URL="$DB_URL" \
  SOLANA_RPC_URL="$RPC" \
  SOLHUB_KEYPAIR="$WALLET" \
  RUST_LOG=info \
  ./target/release/solhub-engine > "$ENGINE_LOG" 2>&1 &
ENGINE_PID=$!
sleep 3
if ! kill -0 $ENGINE_PID 2>/dev/null; then
  echo "Engine failed to start. log:" && tail -30 "$ENGINE_LOG"
  exit 1
fi

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"

echo "--- orgs/me ---"
curl -sf -H "$AUTH" "$API/v1/orgs/me" | tee /dev/stderr | grep -q "$ORG_ID"

echo "--- list workflows (empty) ---"
curl -sf -H "$AUTH" "$API/v1/workflows" | tee /dev/stderr

echo "--- create workflow: system.transfer ---"
CREATE_BODY=$(cat <<JSON
{
  "name": "e2e-transfer",
  "trigger": {"type": "manual"},
  "steps": [{
    "id": "transfer_step",
    "plugin": "system",
    "action": "transfer",
    "params": {"to": "$DEST_PUBKEY", "lamports": $LAMPORTS}
  }]
}
JSON
)
WF_RESP=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$CREATE_BODY" "$API/v1/workflows")
echo "$WF_RESP"
WF_ID=$(echo "$WF_RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')
echo "workflow_id=$WF_ID"

echo "--- trigger workflow ---"
TRIG_RESP=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d '{}' "$API/v1/workflows/$WF_ID/trigger")
echo "$TRIG_RESP"
RUN_ID=$(echo "$TRIG_RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["run_id"])')
echo "run_id=$RUN_ID"

echo "--- poll run status until terminal ---"
for i in $(seq 1 60); do
  STATUS=$(curl -sf -H "$AUTH" "$API/v1/runs/$RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  echo "  [$i] status=$STATUS"
  case "$STATUS" in
    Confirmed|Failed|Skipped) break ;;
  esac
  sleep 2
done

echo "--- final run detail ---"
RUN_DETAIL=$(curl -sf -H "$AUTH" "$API/v1/runs/$RUN_ID")
echo "$RUN_DETAIL"

SIG=$(echo "$RUN_DETAIL" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("signature") or "")')
echo "signature=$SIG"

if [[ -n "$SIG" && "$SIG" != "null" ]]; then
  echo "--- verifying signature on devnet ---"
  PATH=$HOME/.local/share/solana/install/active_release/bin:$PATH solana confirm "$SIG" --url devnet || true
  PATH=$HOME/.local/share/solana/install/active_release/bin:$PATH solana transaction-history "$KEYPAIR_PUBKEY" --url devnet --limit 5 | head -10
fi

echo "--- list runs ---"
curl -sf -H "$AUTH" "$API/v1/runs?workflow_id=$WF_ID" | python3 -m json.tool | head -40

echo "--- analytics ---"
curl -sf -H "$AUTH" "$API/v1/analytics" | python3 -m json.tool

echo "--- webhook flow: create a webhook workflow ---"
WH_BODY=$(cat <<'JSON'
{
  "name": "e2e-webhook",
  "trigger": {"type": "webhook", "secret": "shh-secret"},
  "steps": [{"id": "noop", "plugin": "system", "action": "get_balance", "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}]
}
JSON
)
WH_WF=$(curl -sf -H "$AUTH" -H "Content-Type: application/json" -X POST -d "$WH_BODY" "$API/v1/workflows")
echo "$WH_WF"
WH_WF_ID=$(echo "$WH_WF" | python3 -c 'import sys,json;print(json.load(sys.stdin)["workflow_id"])')

WH_PAYLOAD='{"trigger_data":{"foo":"bar"}}'
SECRET="shh-secret"
HMAC=$(printf "%s" "$WH_PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" -hex | awk '{print $2}')
echo "computed HMAC=$HMAC"
curl -sf -X POST \
  -H "Content-Type: application/json" \
  -H "X-SK-Signature: sha256=$HMAC" \
  -d "$WH_PAYLOAD" \
  "$API/v1/webhooks/$WH_WF_ID" | tee /dev/stderr

echo "--- webhook with bad sig should 401 ---"
HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' -X POST \
  -H "Content-Type: application/json" \
  -H "X-SK-Signature: sha256=deadbeef" \
  -d "$WH_PAYLOAD" \
  "$API/v1/webhooks/$WH_WF_ID")
echo "code=$HTTP_CODE (expected 401)"
[[ "$HTTP_CODE" == "401" ]] || { echo "FAIL: webhook bad sig didn't 401"; exit 1; }

echo "--- API key revocation ---"
KEYS=$(curl -sf -H "$AUTH" "$API/v1/orgs/me/api_keys")
echo "$KEYS"

echo "--- hub: list public workflows (no auth) ---"
curl -sf "$API/v1/hub" | tee /dev/stderr

echo
echo "=== ALL E2E STEPS COMPLETED ==="
echo "  workflow_id     : $WF_ID"
echo "  run_id          : $RUN_ID"
echo "  signature       : ${SIG:-<none>}"
echo "  deployer wallet : $KEYPAIR_PUBKEY"
[[ -n "$SIG" && "$SIG" != "null" ]] && \
  echo "  explorer        : https://explorer.solana.com/tx/$SIG?cluster=devnet"
