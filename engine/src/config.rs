use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub solana_rpc_url: String,
    pub solana_ws_url: Option<String>,
    pub api_port: u16,
    pub turnkey_api_key: Option<String>,
    pub turnkey_org_id: Option<String>,
    pub jito_block_engine_url: Option<String>,
    pub yellowstone_endpoint: Option<String>,
    pub yellowstone_x_token: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub discord_bot_token: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
