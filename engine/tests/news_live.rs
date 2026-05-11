use engine::plugins::news::NewsPlugin;
use engine::plugins::SolanaKeeperPlugin;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;

/// Live integration test — fetches actual CoinDesk RSS.
/// Run with: cargo test -p engine --test news_live -- --include-ignored
#[tokio::test]
#[ignore = "requires live internet access"]
async fn live_fetch_headlines_returns_at_least_one_item() {
    let plugin = NewsPlugin::new();
    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
    let params = json!({"limit": 3});

    let result = plugin
        .read("fetch_headlines", &params, &rpc)
        .await
        .expect("live fetch_headlines should succeed");

    let items = result["items"].as_array().expect("items should be an array");
    assert!(!items.is_empty(), "should have at least one headline");
    println!("First headline: {}", items[0]["title"]);
}
