use anyhow::Result;
use clap::Parser;
use cli::{add_user::add_user, delete_user::delete_user, serve::serve, Commands};
use config::{load_config, Configuration};

use crate::cli::Cli;

pub mod api;
pub mod cli;
pub mod config;
pub mod listener;
pub mod state;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let cfg = load_config()?;
    let cfg: &'static mut Configuration = Box::leak(Box::new(cfg));

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {} => serve(cfg).await,
        Commands::AddUser { name, max_tunnels } => add_user(cfg, name, max_tunnels),
        Commands::DeleteUser { name_or_key } => delete_user(cfg, name_or_key),
    }
}

fn init_logging() {
    if cfg!(debug_assertions) {
        std::env::set_var("RUST_LOG", "debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    env_logger::init();
}
