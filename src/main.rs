#![feature(option_insert)]

use rmig_core::cli::{AppRmigCli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppRmigCli::default().init().execute().await?;
    Ok(())
}

