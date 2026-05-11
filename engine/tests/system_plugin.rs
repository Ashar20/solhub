use engine::plugins::{system::SystemPlugin, ActionType, SolanaKeeperPlugin};

#[test]
fn system_plugin_has_four_actions() {
    let p = SystemPlugin::new();
    let actions = p.actions();
    assert_eq!(
        actions.len(),
        4,
        "expected 4 actions (transfer, memo, get_balance, batch_transfer), got: {:?}",
        actions.iter().map(|a| &a.id).collect::<Vec<_>>()
    );
}

#[test]
fn system_plugin_action_ids_are_correct() {
    let p = SystemPlugin::new();
    let ids: Vec<String> = p.actions().into_iter().map(|a| a.id).collect();
    assert!(ids.contains(&"transfer".to_string()), "missing 'transfer'");
    assert!(ids.contains(&"memo".to_string()), "missing 'memo'");
    assert!(ids.contains(&"get_balance".to_string()), "missing 'get_balance'");
    assert!(ids.contains(&"batch_transfer".to_string()), "missing 'batch_transfer'");
}

#[test]
fn transfer_and_memo_are_transaction_actions() {
    let p = SystemPlugin::new();
    for action in p.actions() {
        match action.id.as_str() {
            "transfer" | "memo" | "batch_transfer" => {
                assert_eq!(
                    action.action_type,
                    ActionType::Transaction,
                    "action '{}' should be Transaction type",
                    action.id
                );
            }
            "get_balance" => {
                assert_eq!(
                    action.action_type,
                    ActionType::ReadOnly,
                    "action 'get_balance' should be ReadOnly type"
                );
            }
            other => panic!("unexpected action id: {}", other),
        }
    }
}

#[test]
fn transfer_schema_requires_to_and_lamports() {
    let p = SystemPlugin::new();
    let transfer = p.actions().into_iter().find(|a| a.id == "transfer").unwrap();
    let schema = &transfer.params_schema;

    let required = schema["required"].as_array().expect("required must be array");
    let required_ids: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(required_ids.contains(&"to"), "'to' must be required");
    assert!(required_ids.contains(&"lamports"), "'lamports' must be required");

    let props = &schema["properties"];
    assert!(props.get("to").is_some(), "missing 'to' in properties");
    assert!(props.get("lamports").is_some(), "missing 'lamports' in properties");
}

#[test]
fn memo_schema_requires_text() {
    let p = SystemPlugin::new();
    let memo = p.actions().into_iter().find(|a| a.id == "memo").unwrap();
    let required = memo.params_schema["required"]
        .as_array()
        .expect("required must be array");
    assert!(required.iter().any(|v| v == "text"), "'text' must be required in memo");
}

#[test]
fn get_balance_schema_requires_account() {
    let p = SystemPlugin::new();
    let bal = p.actions().into_iter().find(|a| a.id == "get_balance").unwrap();
    let required = bal.params_schema["required"]
        .as_array()
        .expect("required must be array");
    assert!(
        required.iter().any(|v| v == "account"),
        "'account' must be required in get_balance"
    );
}

#[test]
fn system_plugin_is_registered_in_default_registry() {
    let reg = engine::plugins::PluginRegistry::default();
    let plugin = reg.get("system");
    assert!(plugin.is_some(), "system plugin must be registered in PluginRegistry::default()");
    assert_eq!(plugin.unwrap().id(), "system");
}
