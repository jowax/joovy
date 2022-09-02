mod client;
mod command_context;
mod command_handler;
mod commands;
mod db;
mod store;

use anyhow::Result;
use tracing::instrument;

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    logger::init().expect("Failed to init logger");
    let conn = db::init().await?;
    client::run(conn).await?;

    Ok(())
}
