use anyhow::{bail, Result};
use config::load_config;
use log::info;
use tokio::{signal::ctrl_c, sync::oneshot::channel};

pub mod api;
pub mod config;
pub mod worker;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let cfg = load_config()?;

    if cfg.tunnels.is_empty() {
        bail!("no tunnels defined in config.toml");
    } else {
        info!("booting with {} tunnels...", cfg.tunnels.len());
    }

    info!("contacting edge server at {}...", cfg.edge);

    if !api::check_authorization(&cfg).await? {
        bail!("failed to authorize with edge server");
    }

    let worker_port = api::connect(&cfg).await?;

    info!(
        "connecting to edge worker server at port {}...",
        worker_port
    );

    let (tx, rx) = channel();
    worker::start_workers(&cfg, worker_port, rx).await?;

    for (id, tunnel) in &cfg.tunnels {
        let (status, target) = api::create_edge(&cfg, id.clone(), tunnel).await?;

        if status == "ok" {
            info!(
                "tunnel {id} (proto={:?}, mode={:?}) created successfully -> {}",
                tunnel.protocol, tunnel.mode, target
            );
        } else {
            bail!(
                "failed to create tunnel {id} (proto={:?}, mode={:?}), status: {status}",
                tunnel.protocol,
                tunnel.mode
            );
        }
    }

    ctrl_c().await?;

    info!("shutting down...");

    tx.send(()).unwrap();
    let response = api::goodbye(&cfg).await?;
    info!("edge server said: {response}");

    Ok(())
}

fn init_logging() {
    if cfg!(debug_assertions) {
        std::env::set_var("RUST_LOG", "debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    env_logger::init();
}
