#![feature(option_insert)]

mod configuration_properties;
mod tera_manager;
mod changelogs;
mod cli;
mod driver;
mod error;
mod utils;

use crate::cli::{AppRmigCli};

extern crate serde_yaml;
extern crate log;
extern crate log4rs;
extern crate lazy_static;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppRmigCli::default().init().run().await?;
    Ok(())
}

