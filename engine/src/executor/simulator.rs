use async_trait::async_trait;
use solana_sdk::transaction::VersionedTransaction;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub units_consumed: u64,
    pub logs: Vec<String>,
}

#[async_trait]
pub trait Simulator: Send + Sync {
    async fn simulate(&self, tx: &VersionedTransaction) -> anyhow::Result<SimulationResult>;
}

// ---------------------------------------------------------------------------
// RPC-backed simulator
// ---------------------------------------------------------------------------

/// Wraps `solana_client::nonblocking::rpc_client::RpcClient` to simulate
/// transactions against a live Solana node.
pub struct RpcSimulator {
    pub rpc: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
}

#[async_trait]
impl Simulator for RpcSimulator {
    async fn simulate(&self, tx: &VersionedTransaction) -> anyhow::Result<SimulationResult> {
        let res = self.rpc.simulate_transaction(tx).await?;
        if let Some(err) = res.value.err {
            anyhow::bail!("simulation failed: {:?}", err);
        }
        Ok(SimulationResult {
            units_consumed: res.value.units_consumed.unwrap_or(0),
            logs: res.value.logs.unwrap_or_default(),
        })
    }
}

// ---------------------------------------------------------------------------
// Mock simulator (for tests)
// ---------------------------------------------------------------------------

pub struct MockSimulator {
    pub units: u64,
}

#[async_trait]
impl Simulator for MockSimulator {
    async fn simulate(&self, _tx: &VersionedTransaction) -> anyhow::Result<SimulationResult> {
        Ok(SimulationResult {
            units_consumed: self.units,
            logs: vec![],
        })
    }
}

/// A mock simulator that always returns an error.
pub struct FailingSimulator;

#[async_trait]
impl Simulator for FailingSimulator {
    async fn simulate(&self, _tx: &VersionedTransaction) -> anyhow::Result<SimulationResult> {
        anyhow::bail!("simulation failure (mock)")
    }
}
