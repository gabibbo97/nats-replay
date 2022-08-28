mod config;
mod info;
mod record;
mod replay;

use tracing::{debug, instrument, info};

use crate::{record::do_record, replay::do_replay, info::do_get_stream_info};

#[instrument(skip_all)]
pub async fn connect_to_broker(config: &config::Config) -> anyhow::Result<async_nats::Client> {
    let client = async_nats::connect(&config.nats_address).await?;
    info!(address = config.nats_address, "Connected to NATS");
    Ok(client)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load config
    let config_path = std::env::var("CONFIG_PATH").unwrap_or("config.json".to_string());
    let config = tokio::fs::read_to_string(&config_path).await?;
    let config = serde_json::from_str::<config::Config>(&config)?;
    debug!("Loaded config: {:?}", config);
    
    // Dispatch command
    match config.mode {
        config::OperationMode::Info => do_get_stream_info(&config).await?,
        config::OperationMode::Record { .. } => do_record(&config).await?,
        config::OperationMode::Replay { .. } => do_replay(&config).await?,
    };
    Ok(())
}
