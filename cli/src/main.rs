use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;

#[derive(Parser)]
#[command(name = "skh", version, about = "SolHub CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Authentication management
    Auth(commands::auth::AuthCmd),
    /// Workflow management
    Workflow(commands::workflow::WorkflowCmd),
    /// Run inspection
    Run(commands::run::RunCmd),
    /// Direct one-shot execution
    Execute(commands::execute::ExecuteCmd),
    /// Billing
    Billing(commands::billing::BillingCmd),
    /// CLI configuration
    Config(commands::config_cmd::ConfigCmd),
    /// x402 payment-gated workflow calls
    X402(commands::x402::X402PayArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Auth(c) => commands::auth::run(c).await,
        Cmd::Workflow(c) => commands::workflow::run(c).await,
        Cmd::Run(c) => commands::run::run(c).await,
        Cmd::Execute(c) => commands::execute::run(c).await,
        Cmd::Billing(c) => commands::billing::run(c).await,
        Cmd::Config(c) => commands::config_cmd::run(c).await,
        Cmd::X402(c) => commands::x402::run(c).await,
    }
}
