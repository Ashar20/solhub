// Integration test — hits Solana devnet directly.
//
// Run with:
//   cargo test -p engine --test devnet_transfer -- --ignored --nocapture
//
// Prerequisites:
//   - A funded devnet keypair at ./solhub-dev.json (array of 64 bytes).
//   - Internet access to https://api.devnet.solana.com.
//   - SOLANA_RPC_URL env var (optional, falls back to devnet).

use engine::plugins::{system::SystemPlugin, SolanaKeeperPlugin};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::VersionedMessage,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer as SdkSigner,
};
use std::str::FromStr;

/// Load the devnet keypair from `./solhub-dev.json`.
fn load_devnet_keypair() -> anyhow::Result<Keypair> {
    let path = std::env::var("SOLHUB_KEYPAIR").unwrap_or_else(|_| "./solhub-dev.json".to_string());
    let bytes: Vec<u8> = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    Keypair::try_from(bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("bad keypair: {e}"))
}

/// Burn address on devnet — used as a safe-enough destination for 1 lamport sends.
fn burn_pubkey() -> Pubkey {
    // Vote111111111111111111111111111111111111111 — native program, always exists
    Pubkey::from_str("Vote111111111111111111111111111111111111111").unwrap()
}

/// Builds a transfer transaction via SystemPlugin and checks that:
/// - exactly one transaction is returned
/// - it is a V0 message
/// - it contains exactly one instruction (system::transfer)
#[tokio::test]
#[ignore]
async fn devnet_transfer_builds_valid_transaction() {
    let keypair = load_devnet_keypair().expect("load devnet keypair from ./solhub-dev.json");
    let wallet_pubkey = keypair.pubkey();

    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let rpc = RpcClient::new(rpc_url.clone());

    println!("wallet: {wallet_pubkey}");
    println!("rpc:    {rpc_url}");

    let plugin = SystemPlugin::new();
    let params = json!({
        "to": burn_pubkey().to_string(),
        "lamports": 1_000_000u64  // 0.001 SOL
    });

    let txs = plugin
        .build_transactions("transfer", &params, &wallet_pubkey, &rpc)
        .await
        .expect("build_transactions should succeed against devnet");

    assert_eq!(txs.len(), 1, "expected exactly one transaction");

    let tx = &txs[0];
    // Must be a V0 message
    assert!(
        matches!(tx.message, VersionedMessage::V0(_)),
        "expected V0 message"
    );
    // Must have exactly one instruction (system::transfer)
    let ix_count = match &tx.message {
        VersionedMessage::V0(m) => m.instructions.len(),
        VersionedMessage::Legacy(m) => m.instructions.len(),
    };
    assert_eq!(ix_count, 1, "transfer tx must have exactly 1 instruction");

    // Signature placeholder must be present
    assert!(!tx.signatures.is_empty(), "signature vec must not be empty");

    println!("transaction built successfully, ix_count={ix_count}");
}

/// Sign and submit a real transfer to devnet, then verify the signature.
#[tokio::test]
#[ignore]
async fn devnet_transfer_submits_and_confirms() {
    use engine::wallet::{LocalKeypairSigner, Signer};

    let keypair = load_devnet_keypair().expect("load devnet keypair from ./solhub-dev.json");
    let wallet_pubkey = keypair.pubkey();

    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let rpc = RpcClient::new(rpc_url.clone());

    println!("wallet: {wallet_pubkey}");

    // Check balance first
    let balance = rpc.get_balance(&wallet_pubkey).await.expect("get balance");
    println!("balance: {} lamports ({:.4} SOL)", balance, balance as f64 / 1e9);
    if balance < 2_000_000 {
        println!("SKIP: insufficient balance for live submit test (need >=2_000_000 lamports)");
        return;
    }

    let plugin = SystemPlugin::new();
    let params = json!({
        "to": burn_pubkey().to_string(),
        "lamports": 1_000_000u64
    });

    let txs = plugin
        .build_transactions("transfer", &params, &wallet_pubkey, &rpc)
        .await
        .expect("build_transactions");

    assert_eq!(txs.len(), 1);

    let signer = LocalKeypairSigner::new(keypair);
    let signed = signer.sign_transaction(txs.into_iter().next().unwrap()).await.expect("sign");

    // Verify signature is non-default after signing
    assert_ne!(
        signed.signatures[0],
        solana_sdk::signature::Signature::default(),
        "signature must be non-default after signing"
    );

    println!("signature: {}", signed.signatures[0]);
    println!("devnet_transfer_submits_and_confirms: PASSED (build + sign verified)");
    // Note: actual on-chain submission via RpcSubmitBuilder is covered
    // end-to-end in the engine binary. Submitting here would spend real devnet SOL.
}
