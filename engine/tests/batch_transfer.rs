/// Integration tests for `system.batch_transfer`.
///
/// These tests validate param parsing and instruction count without hitting a
/// live RPC; blockhash fetches are what require the network, so the actual
/// `build_transactions` call is skipped in the offline case by testing the
/// param-validation paths directly (which short-circuit before any network
/// call).  The devnet smoke test is in `devnet_batch.rs` and is `#[ignore]`d.
use engine::plugins::{system::SystemPlugin, PluginError, SolanaKeeperPlugin};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;

fn rpc() -> solana_client::nonblocking::rpc_client::RpcClient {
    solana_client::nonblocking::rpc_client::RpcClient::new(
        "http://localhost:8899".to_string(),
    )
}

#[tokio::test]
async fn batch_transfer_rejects_missing_transfers_key() {
    let p = SystemPlugin::new();
    let wallet = Pubkey::new_unique();
    let err = p
        .build_transactions("batch_transfer", &json!({}), &wallet, &rpc())
        .await
        .unwrap_err();
    assert!(
        matches!(err, PluginError::InvalidParam(_)),
        "expected InvalidParam, got: {err}"
    );
}

#[tokio::test]
async fn batch_transfer_rejects_empty_array() {
    let p = SystemPlugin::new();
    let wallet = Pubkey::new_unique();
    let err = p
        .build_transactions(
            "batch_transfer",
            &json!({"transfers": []}),
            &wallet,
            &rpc(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, PluginError::InvalidParam(_)),
        "expected InvalidParam, got: {err}"
    );
}

#[tokio::test]
async fn batch_transfer_rejects_16_transfers() {
    let p = SystemPlugin::new();
    let wallet = Pubkey::new_unique();
    let recipient = Pubkey::new_unique().to_string();
    let transfers: Vec<serde_json::Value> = (0..16)
        .map(|_| json!({"to": recipient, "lamports": 1000u64}))
        .collect();

    let err = p
        .build_transactions(
            "batch_transfer",
            &json!({"transfers": transfers}),
            &wallet,
            &rpc(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, PluginError::InvalidParam(_)),
        "expected InvalidParam, got: {err}"
    );
}

#[tokio::test]
async fn batch_transfer_rejects_invalid_pubkey_in_entry() {
    let p = SystemPlugin::new();
    let wallet = Pubkey::new_unique();
    let params = json!({
        "transfers": [
            {"to": "not-a-real-pubkey", "lamports": 1000u64}
        ]
    });
    let err = p
        .build_transactions("batch_transfer", &params, &wallet, &rpc())
        .await
        .unwrap_err();
    assert!(
        matches!(err, PluginError::InvalidParam(_)),
        "expected InvalidParam, got: {err}"
    );
}
