#!/usr/bin/env bash
# x402 end-to-end demo script
#
# Demonstrates the full x402 payment-gated hub call flow against a running
# SolHub API and Solana devnet.
#
# Prerequisites:
#   - SolHub API running locally (default: http://localhost:8080)
#   - solana CLI installed and configured for devnet
#   - A funded devnet keypair at $KEYPAIR_PATH (defaults to solhub-dev.json)
#   - jq installed
#
# Usage:
#   ./tests/e2e/x402_demo.sh
#
# Optional env vars:
#   API_URL         - default: http://localhost:8080
#   API_KEY         - Bearer token for the org (required for auth'd routes)
#   KEYPAIR_PATH    - path to Solana keypair JSON (default: solhub-dev.json)
#   TREASURY        - recipient pubkey (default: FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb)
#   WORKFLOW_ID     - existing public workflow UUID (optional; script creates one if absent)
#   FEE_LAMPORTS    - fee to set on workflow (default: 100000 = 0.0001 SOL)

set -euo pipefail

API_URL="${API_URL:-http://localhost:8080}"
API_KEY="${API_KEY:-}"
KEYPAIR_PATH="${KEYPAIR_PATH:-solhub-dev.json}"
TREASURY="${TREASURY:-FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb}"
FEE_LAMPORTS="${FEE_LAMPORTS:-100000}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${GREEN}[x402]${NC} $*"; }
warn()    { echo -e "${YELLOW}[x402]${NC} $*"; }
die()     { echo -e "${RED}[x402 FAIL]${NC} $*" >&2; exit 1; }

check_dep() { command -v "$1" >/dev/null 2>&1 || die "Missing dependency: $1"; }

check_dep jq
check_dep curl
check_dep solana

# ---------------------------------------------------------------------------
# 1. Health check
# ---------------------------------------------------------------------------
info "1. Health check..."
HEALTH=$(curl -sf "${API_URL}/health") || die "API not reachable at ${API_URL}"
[ "$HEALTH" = "ok" ] || die "Unexpected health response: $HEALTH"
info "   API is up."

if [ -z "$API_KEY" ]; then
    die "API_KEY must be set. Create an org and API key first (e.g. via skh auth login)."
fi

AUTH_HEADER="Authorization: Bearer ${API_KEY}"

# ---------------------------------------------------------------------------
# 2. Create or reuse workflow
# ---------------------------------------------------------------------------
if [ -z "${WORKFLOW_ID:-}" ]; then
    info "2. Creating a new workflow..."
    CREATE_RESP=$(curl -sf -X POST "${API_URL}/v1/workflows" \
        -H "$AUTH_HEADER" \
        -H "Content-Type: application/json" \
        -d '{
              "name": "x402-demo-workflow",
              "trigger": {"type": "manual"},
              "steps": [{"plugin": "system", "action": "get_balance",
                         "params": {"account": "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"}}]
            }') || die "Failed to create workflow"
    WORKFLOW_ID=$(echo "$CREATE_RESP" | jq -r '.workflow_id // .id') \
        || die "Could not parse workflow_id from: $CREATE_RESP"
    info "   Created workflow: $WORKFLOW_ID"
else
    info "2. Using existing workflow: $WORKFLOW_ID"
fi

# ---------------------------------------------------------------------------
# 3. Publish workflow with fee
# ---------------------------------------------------------------------------
info "3. Publishing workflow with fee ${FEE_LAMPORTS} lamports..."
# fee_per_execution_usdc is interpreted as lamports for x402 MVP
FEE_USDC=$(awk -v n="$FEE_LAMPORTS" 'BEGIN { printf "%.6f", n/1000000 }')
PUBLISH_RESP=$(curl -sf -X POST "${API_URL}/v1/hub/publish" \
    -H "$AUTH_HEADER" \
    -H "Content-Type: application/json" \
    -d "{
          \"workflow_id\": \"${WORKFLOW_ID}\",
          \"fee_per_execution_usdc\": ${FEE_USDC}
        }") || die "Failed to publish workflow"
info "   Published: $(echo "$PUBLISH_RESP" | jq -r '.id')"

# ---------------------------------------------------------------------------
# 4. Call without payment — expect 402
# ---------------------------------------------------------------------------
info "4. Calling workflow without X-PAYMENT header (expect 402)..."
STATUS_402=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    "${API_URL}/v1/hub/${WORKFLOW_ID}/call" \
    -H "$AUTH_HEADER")
[ "$STATUS_402" = "402" ] || die "Expected 402, got ${STATUS_402}"
info "   Got 402 as expected."

# ---------------------------------------------------------------------------
# 5. Fetch payment_info
# ---------------------------------------------------------------------------
info "5. Fetching payment requirements..."
PAY_INFO=$(curl -sf "${API_URL}/v1/hub/${WORKFLOW_ID}/payment_info") \
    || die "Failed to fetch payment_info"
AMOUNT=$(echo "$PAY_INFO" | jq -r '.amount_lamports')
RECIPIENT=$(echo "$PAY_INFO" | jq -r '.recipient')
info "   Required: ${AMOUNT} lamports → ${RECIPIENT}"

# ---------------------------------------------------------------------------
# 6. Send SOL on devnet
# ---------------------------------------------------------------------------
info "6. Sending ${AMOUNT} lamports to ${RECIPIENT} on devnet..."
solana config set --url devnet >/dev/null 2>&1 || true

TX_SIG=$(solana transfer \
    --keypair "$KEYPAIR_PATH" \
    --allow-unfunded-recipient \
    "$RECIPIENT" \
    "$(awk -v n="$AMOUNT" 'BEGIN { printf "%.9f", n/1000000000 }')" \
    2>&1 | grep "Signature:" | awk '{print $2}')

[ -n "$TX_SIG" ] || die "Transfer failed — check keypair balance on devnet"
info "   TX signature: ${TX_SIG}"
info "   Waiting 10s for confirmation..."
sleep 10

# ---------------------------------------------------------------------------
# 7. Call workflow with X-PAYMENT header
# ---------------------------------------------------------------------------
info "7. Calling workflow with X-PAYMENT header..."
CALL_RESP=$(curl -sf -X POST \
    "${API_URL}/v1/hub/${WORKFLOW_ID}/call" \
    -H "$AUTH_HEADER" \
    -H "x-payment: solana:devnet:tx:${TX_SIG}") \
    || die "Workflow call failed. Full response: $(curl -s -X POST \
        "${API_URL}/v1/hub/${WORKFLOW_ID}/call" \
        -H "$AUTH_HEADER" \
        -H "x-payment: solana:devnet:tx:${TX_SIG}")"

RUN_ID=$(echo "$CALL_RESP" | jq -r '.run_id') \
    || die "Could not parse run_id from: $CALL_RESP"
PAY_SIG=$(echo "$CALL_RESP" | jq -r '.payment_signature')
info "   Run created: ${RUN_ID}"
info "   Payment signature confirmed: ${PAY_SIG}"

# ---------------------------------------------------------------------------
# 8. Verify replay is rejected
# ---------------------------------------------------------------------------
info "8. Re-sending same signature (replay) — expect 409..."
REPLAY_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    "${API_URL}/v1/hub/${WORKFLOW_ID}/call" \
    -H "$AUTH_HEADER" \
    -H "x-payment: solana:devnet:tx:${TX_SIG}")
[ "$REPLAY_STATUS" = "409" ] || die "Expected 409 on replay, got ${REPLAY_STATUS}"
info "   Got 409 (replay rejected) as expected."

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
info ""
info "x402 demo completed successfully!"
info "  workflow_id:        $WORKFLOW_ID"
info "  payment_signature:  $TX_SIG"
info "  run_id:             $RUN_ID"
