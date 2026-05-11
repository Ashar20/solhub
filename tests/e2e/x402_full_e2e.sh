#!/usr/bin/env bash
# Full x402 E2E: starts API + engine, mints a fresh payer keypair funded from
# solhub-dev, treasures payments to solhub-dev, runs the x402 demo flow.
set -euo pipefail
cd "$(dirname "$0")/../.."

REPO="$(pwd)"
TREASURY_WALLET="$REPO/solhub-dev.json"
PAYER_WALLET="$REPO/tmp/x402-payer.json"
API_PORT="${API_PORT:-18083}"
DB_PATH="$REPO/solhub-x402.db"

mkdir -p "$REPO/tmp"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
DB_URL="sqlite:$DB_PATH?mode=rwc"

export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
TREASURY_PUBKEY="$(solana-keygen pubkey "$TREASURY_WALLET")"

cleanup() {
  echo "--- cleanup ---"
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${ENGINE_PID:-}" ]] && kill "$ENGINE_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

echo "--- mint fresh payer keypair ---"
solana-keygen new --no-bip39-passphrase --silent --force --outfile "$PAYER_WALLET" > /dev/null
PAYER_PUBKEY="$(solana-keygen pubkey "$PAYER_WALLET")"
echo "  treasury: $TREASURY_PUBKEY"
echo "  payer:    $PAYER_PUBKEY"

echo "--- fund payer with 0.05 SOL from treasury ---"
solana transfer --keypair "$TREASURY_WALLET" --url devnet --allow-unfunded-recipient "$PAYER_PUBKEY" 0.05 --commitment confirmed > /dev/null
PAYER_BAL=$(solana balance "$PAYER_PUBKEY" --url devnet)
echo "  payer balance: $PAYER_BAL"

echo "--- start API ---"
DATABASE_URL="$DB_URL" \
  API_PORT=$API_PORT \
  SOLANA_RPC_URL="https://api.devnet.solana.com" \
  SOLHUB_TREASURY="$TREASURY_PUBKEY" \
  RUST_LOG=warn \
  ./target/release/solhub-api > "$REPO/tmp/x402-api.log" 2>&1 &
API_PID=$!
sleep 2
curl -sf "http://localhost:$API_PORT/health" >/dev/null

RAW_KEY="sk_x402_$(date +%s)"
KEY_HASH=$(printf "%s" "$RAW_KEY" | sha256sum | awk '{print $1}')
ORG_ID=$(uuidgen)
KEY_ID=$(uuidgen)
NOW=$(date +%s)
sqlite3 "$DB_PATH" <<SQL
INSERT INTO organizations (id, name, wallet_address, credits_usdc, created_at) VALUES ('$ORG_ID', 'x402-org', NULL, 0, $NOW);
INSERT INTO api_keys (id, org_id, key_hash, name, last_used_at, created_at, revoked_at) VALUES ('$KEY_ID', '$ORG_ID', '$KEY_HASH', 'x402', NULL, $NOW, NULL);
SQL

echo "--- start engine ---"
DATABASE_URL="$DB_URL" SOLANA_RPC_URL="https://api.devnet.solana.com" SOLHUB_KEYPAIR="$TREASURY_WALLET" RUST_LOG=warn ./target/release/solhub-engine > "$REPO/tmp/x402-engine.log" 2>&1 &
ENGINE_PID=$!
sleep 2

export API_URL="http://localhost:$API_PORT"
export API_KEY="$RAW_KEY"
export KEYPAIR_PATH="$PAYER_WALLET"        # pay from the fresh payer
export TREASURY="$TREASURY_PUBKEY"
export FEE_LAMPORTS="100000"

bash "$REPO/tests/e2e/x402_demo.sh"
echo
echo "=== x402 FULL E2E PASSED ==="
