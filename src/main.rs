mod client;
mod commands;
mod command_handler;
mod command_context;

use anyhow::Result;
use tracing::instrument;

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    logger::init().expect("Failed to init logger");
    client::run().await?;

    Ok(())
}
