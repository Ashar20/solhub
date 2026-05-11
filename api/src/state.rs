use db::Db;
use engine::plugins::PluginRegistry;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub plugins: Arc<PluginRegistry>,
    pub manual_triggers: tokio::sync::broadcast::Sender<Uuid>,
}

impl AppState {
    pub fn new(db: Db, plugins: Arc<PluginRegistry>) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(256);
        Self { db, plugins, manual_triggers: tx }
    }
}
