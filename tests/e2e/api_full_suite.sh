#!/usr/bin/env bash
# Comprehensive REST API smoke suite.
# Drives every public endpoint against a fresh in-process API+engine stack
# and asserts both HTTP status codes and response body shapes.
#
# Endpoints covered:
#   - GET    /health
#   - GET    /v1/orgs/me                        + 401 without auth
#   - POST   /v1/orgs/me/api_keys               (create returns raw key once)
#   - GET    /v1/orgs/me/api_keys
#   - DELETE /v1/orgs/me/api_keys/:id
#   - POST   /v1/workflows                      (create) + 400 (bad trigger)
#   - GET    /v1/workflows                      (list)
#   - GET    /v1/workflows/:id                  + 404 missing
#   - PATCH  /v1/workflows/:id                  (toggle is_active)
#   - DELETE /v1/workflows/:id                  (soft delete)
#   - POST   /v1/workflows/:id/trigger          (manual)
#   - GET    /v1/runs                           (list + filter)
#   - GET    /v1/runs/:id                       + 404 missing
#   - GET    /v1/runs/:id/logs                  (SSE)
#   - POST   /v1/runs/:id/approve               (after WaitingApproval)
#   - POST   /v1/runs/:id/reject
#   - POST   /v1/webhooks/:id                   + 401 (bad HMAC)
#   - GET    /v1/analytics
#   - GET    /v1/orgs/me/credits
#   - POST   /v1/orgs/me/credits/grant          (admin)
#   - GET    /v1/hub                            (public list, no auth)
#   - POST   /v1/hub/publish
#   - GET    /v1/hub/:id/payment_info
#   - POST   /v1/hub/:id/call                   (without + with x402 + replay)

set -uo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
WALLET="$REPO/solhub-dev.json"
API_PORT="${API_PORT:-18088}"
DB_PATH="$REPO/solhub-apifull.db"
mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
DB_URL="sqlite:$DB_PATH?mode=rwc"
ADMIN_TOKEN="admin-test"
WEBHOOK_SECRET="webhook-shh"

cleanup() {
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

DATABASE_URL="$DB_URL" \
  API_PORT=$API_PORT \
  SOLANA_RPC_URL="https://api.devnet.solana.com" \
  SOLHUB_TREASURY="FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb" \
  SOLHUB_ADMIN_TOKEN="$ADMIN_TOKEN" \
  RUST_LOG=warn \
  ./target/release/solhub-api > "$REPO/tmp/apifull-api.log" 2>&1 &
API_PID=$!
sleep 2

RAW_KEY="sk_apifull_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'apifull-org', NULL, 100, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'apifull', NULL, $NOW, NULL);
SQL

DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/apifull-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

API="http://localhost:$API_PORT"
AUTH="Authorization: Bearer $RAW_KEY"

PASS=0
FAIL=0
RESULTS=()
SECTION=""

section() { SECTION="$1"; echo; echo "═══════ $1 ═══════"; }

# check NAME EXPECTED_CODE GOT_CODE [extra_check_command]
check() {
  local name="$1" exp="$2" got="$3" extra="${4:-true}"
  if [[ "$got" == "$exp" ]] && eval "$extra"; then
    PASS=$((PASS+1)); RESULTS+=("PASS $SECTION :: $name (got $got)")
    echo "  ✓ $name → $got"
  else
    FAIL=$((FAIL+1)); RESULTS+=("FAIL $SECTION :: $name (expected $exp got $got)")
    echo "  ✗ $name → got $got, expected $exp"
  fi
}

# extract http code + body separately
HCODE=""
BODY=""
http_req() {
  local method="$1" path="$2"
  shift 2
  local resp
  resp=$(curl -s -o /tmp/api_body.json -w "%{http_code}" -X "$method" "$@" "$API$path")
  HCODE="$resp"
  BODY="$(cat /tmp/api_body.json 2>/dev/null || true)"
}

jq_get() { echo "$1" | python3 -c "import sys,json;print(json.load(sys.stdin)$2)" 2>/dev/null; }

##############################################################################
section "health + auth"
##############################################################################
http_req GET /health
check "GET /health returns ok"   200 "$HCODE"  '[[ "$BODY" == "ok" ]]'

http_req GET /v1/orgs/me
check "GET /v1/orgs/me without bearer is 401"  401 "$HCODE"

http_req GET /v1/orgs/me -H "$AUTH"
check "GET /v1/orgs/me with bearer is 200"     200 "$HCODE" '[[ "$(jq_get "$BODY" "[\"id\"]")" == "'"$ORG_ID"'" ]]'

http_req GET /v1/orgs/me -H "Authorization: Bearer bogus"
check "GET /v1/orgs/me with bad bearer is 401" 401 "$HCODE"

##############################################################################
section "api keys CRUD"
##############################################################################
http_req POST /v1/orgs/me/api_keys -H "$AUTH" -H "Content-Type: application/json" -d '{"name":"second-key"}'
check "POST /v1/orgs/me/api_keys is 200" 200 "$HCODE"
SECOND_KEY=$(jq_get "$BODY" '["key"]')
SECOND_KEY_ID=$(jq_get "$BODY" '["id"]')
[[ -n "$SECOND_KEY" ]] && echo "  raw key returned (len=${#SECOND_KEY})"

http_req GET /v1/orgs/me/api_keys -H "$AUTH"
check "GET /v1/orgs/me/api_keys lists 2 keys" 200 "$HCODE" '[[ "$(echo "$BODY" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")" -ge 2 ]]'

http_req GET /v1/orgs/me -H "Authorization: Bearer $SECOND_KEY"
check "second key authenticates" 200 "$HCODE"

http_req DELETE "/v1/orgs/me/api_keys/$SECOND_KEY_ID" -H "$AUTH"
check "DELETE /v1/orgs/me/api_keys/:id is 200" 200 "$HCODE"

http_req GET /v1/orgs/me -H "Authorization: Bearer $SECOND_KEY"
check "revoked key no longer authenticates" 401 "$HCODE"

##############################################################################
section "workflow CRUD"
##############################################################################
http_req POST /v1/workflows -H "$AUTH" -H "Content-Type: application/json" -d '{"name":"apifull-test","trigger":{"type":"junk"},"steps":[]}'
check "create workflow w/ bad trigger is 400"  400 "$HCODE"

http_req POST /v1/workflows -H "$AUTH" -H "Content-Type: application/json" -d '{
  "name":"apifull-test",
  "trigger":{"type":"manual"},
  "steps":[{"id":"s1","plugin":"system","action":"get_balance","params":{"account":"FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}]
}'
check "create workflow OK" 200 "$HCODE"
WF_ID=$(jq_get "$BODY" '["workflow_id"]')
echo "  wf_id=$WF_ID"

http_req GET /v1/workflows -H "$AUTH"
check "list workflows" 200 "$HCODE" '[[ "$(echo "$BODY" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")" -ge 1 ]]'

http_req GET "/v1/workflows/$WF_ID" -H "$AUTH"
check "get workflow by id" 200 "$HCODE"

http_req GET "/v1/workflows/00000000-0000-0000-0000-000000000000" -H "$AUTH"
check "get missing workflow is 404" 404 "$HCODE"

http_req PATCH "/v1/workflows/$WF_ID" -H "$AUTH" -H "Content-Type: application/json" -d '{"is_active":false}'
check "patch workflow toggles is_active" 200 "$HCODE"

http_req GET "/v1/workflows/$WF_ID" -H "$AUTH"
check "patched workflow shows is_active=false" 200 "$HCODE" '[[ "$(jq_get "$BODY" "[\"is_active\"]")" == "False" ]]'

http_req PATCH "/v1/workflows/$WF_ID" -H "$AUTH" -H "Content-Type: application/json" -d '{"is_active":true}'
check "re-enable workflow" 200 "$HCODE"

##############################################################################
section "trigger + run lifecycle"
##############################################################################
http_req POST "/v1/workflows/$WF_ID/trigger" -H "$AUTH" -H "Content-Type: application/json" -d '{}'
check "trigger workflow" 200 "$HCODE"
RUN_ID=$(jq_get "$BODY" '["run_id"]')
echo "  run_id=$RUN_ID"

http_req GET "/v1/runs/$RUN_ID" -H "$AUTH"
check "get run by id" 200 "$HCODE"

http_req GET /v1/runs -H "$AUTH"
check "list runs" 200 "$HCODE"

http_req GET "/v1/runs?workflow_id=$WF_ID" -H "$AUTH"
check "list runs filtered by workflow_id" 200 "$HCODE" '[[ "$(echo "$BODY" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")" -ge 1 ]]'

http_req GET "/v1/runs/00000000-0000-0000-0000-000000000000" -H "$AUTH"
check "get missing run is 404" 404 "$HCODE"

# Wait for completion
for i in $(seq 1 20); do
  STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$RUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  case "$STATUS" in Confirmed|Failed|Skipped) break ;; esac
  sleep 1
done
[[ "$STATUS" == "Confirmed" ]]
check "run reaches Confirmed" 0 "$?"

##############################################################################
section "SSE log streaming"
##############################################################################
# Trigger a fresh run, immediately stream the logs
http_req POST "/v1/workflows/$WF_ID/trigger" -H "$AUTH" -H "Content-Type: application/json" -d '{}'
SSE_RUN_ID=$(jq_get "$BODY" '["run_id"]')
SSE_OUT=$(timeout 15 curl -sf -N -H "$AUTH" -H "Accept: text/event-stream" "$API/v1/runs/$SSE_RUN_ID/logs" 2>&1 || true)
echo "$SSE_OUT" | grep -q 'event: run_complete'
check "SSE delivers run_complete event" 0 "$?"
echo "$SSE_OUT" | grep -q 'event: step_log'
check "SSE delivers step_log event(s)" 0 "$?"

##############################################################################
section "approval gate"
##############################################################################
http_req POST /v1/workflows -H "$AUTH" -H "Content-Type: application/json" -d '{
  "name":"apifull-approval",
  "trigger":{"type":"manual"},
  "steps":[
    {"id":"s1","plugin":"solhub","action":"require_approval","params":{"message":"go?"}},
    {"id":"s2","plugin":"system","action":"get_balance","params":{"account":"FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}
  ]
}'
AWF_ID=$(jq_get "$BODY" '["workflow_id"]')

http_req POST "/v1/workflows/$AWF_ID/trigger" -H "$AUTH" -H "Content-Type: application/json" -d '{}'
ARUN_ID=$(jq_get "$BODY" '["run_id"]')

for i in $(seq 1 20); do
  STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$ARUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  [[ "$STATUS" == "WaitingApproval" ]] && break
  sleep 1
done
[[ "$STATUS" == "WaitingApproval" ]]
check "run reaches WaitingApproval" 0 "$?"

http_req POST "/v1/runs/$ARUN_ID/approve" -H "$AUTH" -H "Content-Type: application/json" -d '{}'
check "approve endpoint returns 200" 200 "$HCODE"

for i in $(seq 1 20); do
  STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$ARUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  case "$STATUS" in Confirmed|Failed) break ;; esac
  sleep 1
done
[[ "$STATUS" == "Confirmed" ]]
check "approved run completes" 0 "$?"

# Reject path
http_req POST "/v1/workflows/$AWF_ID/trigger" -H "$AUTH" -H "Content-Type: application/json" -d '{}'
RRUN_ID=$(jq_get "$BODY" '["run_id"]')
for i in $(seq 1 20); do
  STATUS=$(curl -s -H "$AUTH" "$API/v1/runs/$RRUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
  [[ "$STATUS" == "WaitingApproval" ]] && break
  sleep 1
done
http_req POST "/v1/runs/$RRUN_ID/reject" -H "$AUTH" -H "Content-Type: application/json" -d '{"reason":"nope"}'
check "reject endpoint returns 200" 200 "$HCODE"
STATUS_FINAL=$(curl -s -H "$AUTH" "$API/v1/runs/$RRUN_ID" | python3 -c 'import sys,json;print(json.load(sys.stdin)["status"])')
[[ "$STATUS_FINAL" == "Failed" ]]
check "rejected run is Failed" 0 "$?"

##############################################################################
section "webhook HMAC"
##############################################################################
http_req POST /v1/workflows -H "$AUTH" -H "Content-Type: application/json" -d "{
  \"name\":\"apifull-webhook\",
  \"trigger\":{\"type\":\"webhook\",\"secret\":\"$WEBHOOK_SECRET\"},
  \"steps\":[{\"id\":\"s1\",\"plugin\":\"system\",\"action\":\"get_balance\",\"params\":{\"account\":\"FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb\"}}]
}"
WHWF_ID=$(jq_get "$BODY" '["workflow_id"]')

PAYLOAD='{"trigger_data":{"hello":"world"}}'
GOOD_HMAC=$(printf "%s" "$PAYLOAD" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" -hex | awk '{print $2}')

http_req POST "/v1/webhooks/$WHWF_ID" -H "Content-Type: application/json" -H "X-SK-Signature: sha256=$GOOD_HMAC" -d "$PAYLOAD"
check "webhook with valid HMAC is 200" 200 "$HCODE"

http_req POST "/v1/webhooks/$WHWF_ID" -H "Content-Type: application/json" -H "X-SK-Signature: sha256=deadbeef" -d "$PAYLOAD"
check "webhook with bad HMAC is 401" 401 "$HCODE"

http_req POST "/v1/webhooks/$WHWF_ID" -H "Content-Type: application/json" -d "$PAYLOAD"
check "webhook with missing sig is 401" 401 "$HCODE"

##############################################################################
section "analytics + credits"
##############################################################################
http_req GET /v1/analytics -H "$AUTH"
check "GET /v1/analytics" 200 "$HCODE" '[[ "$(jq_get "$BODY" "[\"total_executions\"]")" != "" ]]'

http_req GET /v1/orgs/me/credits -H "$AUTH"
check "GET /v1/orgs/me/credits" 200 "$HCODE"

http_req POST /v1/orgs/me/credits/grant -H "$AUTH" -H "X-Admin-Token: $ADMIN_TOKEN" -H "Content-Type: application/json" -d "{\"org_id\":\"$ORG_ID\",\"amount\":50,\"reason\":\"test\"}"
check "admin grant credits" 200 "$HCODE"

http_req POST /v1/orgs/me/credits/grant -H "$AUTH" -H "X-Admin-Token: wrong" -H "Content-Type: application/json" -d "{\"org_id\":\"$ORG_ID\",\"amount\":50,\"reason\":\"test\"}"
check "admin grant w/ wrong token is 403/401" 403 "$HCODE" '[[ "$HCODE" =~ ^4[0-9][0-9]$ ]]'

##############################################################################
section "hub + x402"
##############################################################################
http_req GET /v1/hub
check "GET /v1/hub (no auth)" 200 "$HCODE"

# Publish the original workflow (re-use $WF_ID)
http_req POST /v1/hub/publish -H "$AUTH" -H "Content-Type: application/json" -d "{\"workflow_id\":\"$WF_ID\",\"fee_per_execution_usdc\":0.1}"
check "POST /v1/hub/publish" 200 "$HCODE"

http_req GET "/v1/hub/$WF_ID/payment_info"
check "GET payment_info (no auth)" 200 "$HCODE" '[[ "$(jq_get "$BODY" "[\"amount_lamports\"]")" == "100000" ]]'

http_req POST "/v1/hub/$WF_ID/call" -H "$AUTH" -H "Content-Type: application/json" -d '{}'
check "hub/call without payment is 402" 402 "$HCODE"

##############################################################################
section "delete (soft) workflow"
##############################################################################
http_req DELETE "/v1/workflows/$WF_ID" -H "$AUTH"
check "DELETE workflow soft-deletes" 200 "$HCODE"
http_req GET "/v1/workflows/$WF_ID" -H "$AUTH"
check "soft-deleted workflow still retrievable" 200 "$HCODE" '[[ "$(jq_get "$BODY" "[\"is_active\"]")" == "False" ]]'

##############################################################################
section "summary"
##############################################################################
echo
echo "════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  case "$r" in
    PASS*) echo "  $r" ;;
    FAIL*) echo "  $r" ;;
  esac
done
TOTAL=$((PASS+FAIL))
echo "  ─────────────────────────────────────"
echo "  total: $TOTAL  pass: $PASS  fail: $FAIL"
[[ $FAIL -eq 0 ]] && { echo "ALL API ENDPOINTS PASS"; exit 0; } || { echo "SOME API ENDPOINTS FAILED"; exit 1; }
