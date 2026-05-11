use engine::plugins::llm::LlmPlugin;
use engine::plugins::SolanaKeeperPlugin;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;

/// Live integration test — requires OPENAI_API_KEY in the environment.
/// Run with: cargo test -p engine --test llm_live -- --include-ignored
#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
async fn live_complete_returns_text() {
    let key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set for live test");
    assert!(!key.is_empty(), "OPENAI_API_KEY must not be empty");

    let plugin = LlmPlugin::new();
    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
    let params = json!({
        "prompt": "Say exactly: hello solhub",
        "max_tokens": 20,
        "temperature": 0.0
    });

    let result = plugin
        .read("complete", &params, &rpc)
        .await
        .expect("live complete should succeed");

    let text = result["text"].as_str().unwrap_or("");
    assert!(!text.is_empty(), "response text should not be empty");
    println!("LLM response: {}", text);
}
