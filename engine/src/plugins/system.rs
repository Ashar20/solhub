use super::{ActionDefinition, ActionType, PluginError, SolanaKeeperPlugin};
use async_trait::async_trait;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
#[allow(deprecated)]
use solana_sdk::system_instruction;
use solana_sdk::{
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use std::str::FromStr;

pub struct SystemPlugin;

impl SystemPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SolanaKeeperPlugin for SystemPlugin {
    fn id(&self) -> &'static str {
        "system"
    }

    fn name(&self) -> &'static str {
        "System"
    }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "transfer".to_string(),
                name: "SOL Transfer".to_string(),
                description: "Transfer native SOL between accounts".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["to", "lamports"],
                    "properties": {
                        "to": {"type": "string", "description": "destination pubkey (base58)"},
                        "lamports": {"type": "integer", "description": "amount in lamports"}
                    }
                }),
                returns_schema: json!({"signature": "string"}),
            },
            ActionDefinition {
                id: "memo".to_string(),
                name: "Memo".to_string(),
                description: "Attach an SPL Memo to a transaction".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["text"],
                    "properties": {"text": {"type": "string"}}
                }),
                returns_schema: json!({"signature": "string"}),
            },
            ActionDefinition {
                id: "get_balance".to_string(),
                name: "Get Balance".to_string(),
                description: "Read the SOL balance of any account".to_string(),
                action_type: ActionType::ReadOnly,
                params_schema: json!({
                    "type": "object",
                    "required": ["account"],
                    "properties": {
                        "account": {"type": "string", "description": "account pubkey (base58)"}
                    }
                }),
                returns_schema: json!({"lamports": "integer", "sol": "number"}),
            },
            ActionDefinition {
                id: "batch_transfer".to_string(),
                name: "Batch SOL Transfer".to_string(),
                description: "Atomically transfer SOL to multiple recipients in one transaction".to_string(),
                action_type: ActionType::Transaction,
                params_schema: json!({
                    "type": "object",
                    "required": ["transfers"],
                    "properties": {
                        "transfers": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["to", "lamports"],
                                "properties": {
                                    "to": {"type": "string"},
                                    "lamports": {"type": "integer"}
                                }
                            },
                            "minItems": 1,
                            "maxItems": 15
                        }
                    }
                }),
                returns_schema: json!({"signature": "string"}),
            },
        ]
    }

    async fn build_transactions(
        &self,
        action: &str,
        params: &Value,
        wallet_pubkey: &Pubkey,
        rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError> {
        match action {
            "transfer" => {
                let to = params["to"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("to".into()))?;
                let lamports = params["lamports"]
                    .as_u64()
                    .ok_or_else(|| PluginError::InvalidParam("lamports".into()))?;
                let to_pk = Pubkey::from_str(to)
                    .map_err(|_| PluginError::InvalidParam("to: not a pubkey".into()))?;

                let ix = system_instruction::transfer(wallet_pubkey, &to_pk, lamports);
                let recent = rpc
                    .get_latest_blockhash()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                let msg = v0::Message::try_compile(wallet_pubkey, &[ix], &[], recent)
                    .map_err(|e| PluginError::Other(format!("message compile: {e}")))?;
                let tx = VersionedTransaction {
                    signatures: vec![solana_sdk::signature::Signature::default()],
                    message: VersionedMessage::V0(msg),
                };
                Ok(vec![tx])
            }
            "memo" => {
                let text = params["text"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("text".into()))?;
                let memo_program =
                    Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
                        .map_err(|_| PluginError::Other("bad memo program id".into()))?;
                let ix = solana_sdk::instruction::Instruction {
                    program_id: memo_program,
                    accounts: vec![],
                    data: text.as_bytes().to_vec(),
                };
                let recent = rpc
                    .get_latest_blockhash()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                let msg = v0::Message::try_compile(wallet_pubkey, &[ix], &[], recent)
                    .map_err(|e| PluginError::Other(format!("message compile: {e}")))?;
                let tx = VersionedTransaction {
                    signatures: vec![solana_sdk::signature::Signature::default()],
                    message: VersionedMessage::V0(msg),
                };
                Ok(vec![tx])
            }
            "batch_transfer" => {
                let transfers = params["transfers"]
                    .as_array()
                    .ok_or_else(|| PluginError::InvalidParam("transfers".into()))?;

                if transfers.is_empty() {
                    return Err(PluginError::InvalidParam(
                        "transfers: must have at least 1 entry".into(),
                    ));
                }
                if transfers.len() > 15 {
                    return Err(PluginError::InvalidParam(
                        "transfers: exceeds max of 15 entries".into(),
                    ));
                }

                let mut ixs = Vec::with_capacity(transfers.len());
                for (idx, t) in transfers.iter().enumerate() {
                    let to = t["to"].as_str().ok_or_else(|| {
                        PluginError::InvalidParam(format!("transfers[{idx}].to"))
                    })?;
                    let lamports = t["lamports"].as_u64().ok_or_else(|| {
                        PluginError::InvalidParam(format!("transfers[{idx}].lamports"))
                    })?;
                    let to_pk = Pubkey::from_str(to).map_err(|_| {
                        PluginError::InvalidParam(format!("transfers[{idx}].to: not a pubkey"))
                    })?;
                    ixs.push(system_instruction::transfer(wallet_pubkey, &to_pk, lamports));
                }

                let recent = rpc
                    .get_latest_blockhash()
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                let msg = v0::Message::try_compile(wallet_pubkey, &ixs, &[], recent)
                    .map_err(|e| PluginError::Other(format!("message compile: {e}")))?;
                let tx = VersionedTransaction {
                    signatures: vec![solana_sdk::signature::Signature::default()],
                    message: VersionedMessage::V0(msg),
                };
                Ok(vec![tx])
            }
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }

    async fn read(
        &self,
        action: &str,
        params: &Value,
        rpc: &RpcClient,
    ) -> Result<Value, PluginError> {
        match action {
            "get_balance" => {
                let account = params["account"]
                    .as_str()
                    .ok_or_else(|| PluginError::InvalidParam("account".into()))?;
                let pk = Pubkey::from_str(account)
                    .map_err(|_| PluginError::InvalidParam("account: not a pubkey".into()))?;
                let lamports = rpc
                    .get_balance(&pk)
                    .await
                    .map_err(|e| PluginError::Network(e.to_string()))?;
                Ok(json!({"lamports": lamports, "sol": lamports as f64 / 1_000_000_000.0}))
            }
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_plugin_has_correct_id() {
        let p = SystemPlugin::new();
        assert_eq!(p.id(), "system");
        assert_eq!(p.name(), "System");
    }

    #[test]
    fn actions_has_transfer_memo_get_balance_batch_transfer() {
        let p = SystemPlugin::new();
        let actions = p.actions();
        assert_eq!(actions.len(), 4);

        let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"transfer"));
        assert!(ids.contains(&"memo"));
        assert!(ids.contains(&"get_balance"));
        assert!(ids.contains(&"batch_transfer"));
    }

    #[test]
    fn transfer_action_is_transaction_type() {
        let p = SystemPlugin::new();
        let transfer = p.actions().into_iter().find(|a| a.id == "transfer").unwrap();
        assert_eq!(transfer.action_type, ActionType::Transaction);

        let required = transfer.params_schema["required"].as_array().unwrap();
        let required_ids: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(required_ids.contains(&"to"));
        assert!(required_ids.contains(&"lamports"));
    }

    #[test]
    fn get_balance_action_is_readonly_type() {
        let p = SystemPlugin::new();
        let action = p.actions().into_iter().find(|a| a.id == "get_balance").unwrap();
        assert_eq!(action.action_type, ActionType::ReadOnly);

        let required = action.params_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "account"));
    }

    #[tokio::test]
    async fn transfer_missing_to_returns_invalid_param() {
        let p = SystemPlugin::new();
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"lamports": 1000u64});

        let err = p.build_transactions("transfer", &params, &wallet, &rpc).await.unwrap_err();
        assert!(matches!(err, PluginError::InvalidParam(ref s) if s == "to"));
    }

    #[tokio::test]
    async fn transfer_invalid_pubkey_returns_invalid_param() {
        let p = SystemPlugin::new();
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"to": "not-a-pubkey", "lamports": 1000u64});

        let err = p.build_transactions("transfer", &params, &wallet, &rpc).await.unwrap_err();
        assert!(matches!(err, PluginError::InvalidParam(_)));
    }

    #[tokio::test]
    async fn unknown_build_action_returns_error() {
        let p = SystemPlugin::new();
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let err = p.build_transactions("nonexistent", &json!({}), &wallet, &rpc).await.unwrap_err();
        assert!(matches!(err, PluginError::UnknownAction(_)));
    }

    #[tokio::test]
    async fn unknown_read_action_returns_error() {
        let p = SystemPlugin::new();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let err = p.read("nonexistent", &json!({}), &rpc).await.unwrap_err();
        assert!(matches!(err, PluginError::UnknownAction(_)));
    }

    #[tokio::test]
    async fn get_balance_missing_account_returns_invalid_param() {
        let p = SystemPlugin::new();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let err = p.read("get_balance", &json!({}), &rpc).await.unwrap_err();
        assert!(matches!(err, PluginError::InvalidParam(ref s) if s == "account"));
    }

    #[test]
    fn batch_transfer_action_appears_in_actions_list() {
        let p = SystemPlugin::new();
        let actions = p.actions();

        let bt = actions.iter().find(|a| a.id == "batch_transfer");
        assert!(bt.is_some(), "batch_transfer must be in actions list");

        let bt = bt.unwrap();
        assert_eq!(bt.action_type, ActionType::Transaction);

        let required = bt.params_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "transfers"), "'transfers' must be required");

        let transfers_schema = &bt.params_schema["properties"]["transfers"];
        assert_eq!(transfers_schema["minItems"], 1);
        assert_eq!(transfers_schema["maxItems"], 15);
    }

    #[tokio::test]
    async fn batch_transfer_rejects_empty_array() {
        let p = SystemPlugin::new();
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let params = json!({"transfers": []});

        let err = p
            .build_transactions("batch_transfer", &params, &wallet, &rpc)
            .await
            .unwrap_err();
        assert!(
            matches!(err, PluginError::InvalidParam(_)),
            "expected InvalidParam, got: {err}"
        );
    }

    #[tokio::test]
    async fn batch_transfer_rejects_more_than_15() {
        let p = SystemPlugin::new();
        let wallet = Pubkey::new_unique();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

        let recipient = Pubkey::new_unique().to_string();
        let transfers: Vec<serde_json::Value> = (0..16)
            .map(|_| json!({"to": recipient, "lamports": 1000u64}))
            .collect();
        let params = json!({"transfers": transfers});

        let err = p
            .build_transactions("batch_transfer", &params, &wallet, &rpc)
            .await
            .unwrap_err();
        assert!(
            matches!(err, PluginError::InvalidParam(_)),
            "expected InvalidParam, got: {err}"
        );
    }
}
