/// Devnet smoke-test for `system.batch_transfer`.
///
/// This test is marked `#[ignore]` so it is skipped in CI.  Run it manually
/// with a funded devnet wallet:
///
/// ```bash
/// export SOLANA_RPC_URL=https://api.devnet.solana.com
/// export SOLHUB_KEYPAIR=./solhub-dev.json
/// cargo test -p engine --test devnet_batch -- --ignored
/// ```
use engine::plugins::{SolanaKeeperPlugin, system::SystemPlugin};
use serde_json::json;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer as SdkSigner,
};

#[tokio::test]
#[ignore]
async fn batch_transfer_3_recipients_on_devnet() {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let rpc = solana_client::nonblocking::rpc_client::RpcClient::new(rpc_url);

    // Load keypair from SOLHUB_KEYPAIR or generate an ephemeral one (will fail
    // to submit if it has no SOL, but the build step will still work).
    let wallet: Pubkey = if let Ok(path) = std::env::var("SOLHUB_KEYPAIR") {
        let bytes: Vec<u8> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        Keypair::try_from(bytes.as_slice()).unwrap().pubkey()
    } else {
        Keypair::new().pubkey()
    };

    let r1 = Pubkey::new_unique();
    let r2 = Pubkey::new_unique();
    let r3 = Pubkey::new_unique();

    let params = json!({
        "transfers": [
            {"to": r1.to_string(), "lamports": 1000u64},
            {"to": r2.to_string(), "lamports": 1000u64},
            {"to": r3.to_string(), "lamports": 1000u64},
        ]
    });

    let p = SystemPlugin::new();
    let txs = p
        .build_transactions("batch_transfer", &params, &wallet, &rpc)
        .await
        .expect("build_transactions should succeed");

    assert_eq!(txs.len(), 1, "should produce exactly one transaction");

    // Verify the transaction message contains 3 instructions
    use solana_sdk::message::VersionedMessage;
    let msg = match &txs[0].message {
        VersionedMessage::V0(m) => m,
        _ => panic!("expected v0 message"),
    };
    assert_eq!(
        msg.instructions.len(),
        3,
        "transaction should have 3 transfer instructions"
    );
}
