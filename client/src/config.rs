use std::{
    collections::HashMap,
    fs::read_to_string,
    net::{SocketAddr, ToSocketAddrs},
};

use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};
use toml::from_str;

const CONFIG_PATH: &str = if cfg!(debug_assertions) {
    "client/config.toml"
} else {
    "config.toml"
};

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub secret_key: String,
    pub edge: String,
    pub edge_ip: String,
    pub idle_workers: usize,
    pub tunnels: HashMap<String, Tunnel>,
}

#[derive(Deserialize, Serialize)]
pub struct Tunnel {
    pub target: String,
    pub protocol: Protocol,
    pub mode: Mode,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Protocol {
    Tcp,
    HAProxyV1,
    HAProxyV2,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Mode {
    Reverse,
    HolePunch,
}

pub fn load_config() -> Result<Configuration> {
    let file = read_to_string(CONFIG_PATH)?;
    let mut config: Configuration = from_str(&file)?;

    for (id, tunnel) in config.tunnels.iter_mut() {
        // we have to resolve the target to an IP address
        let target = match tunnel.target.parse::<SocketAddr>() {
            Ok(target) => target,
            Err(_) => {
                let server = tunnel
                    .target
                    .to_socket_addrs()
                    .expect("unable to resolve target")
                    .collect::<Vec<SocketAddr>>();

                let target = server
                    .iter()
                    .find(|&addr| addr.is_ipv4())
                    .or_else(|| server.first())
                    .ok_or_else(|| {
                        anyhow!(
                            "unable to resolve target {} for tunnel {}",
                            tunnel.target,
                            id
                        )
                    })?;

                info!("resolved target {} to {}", tunnel.target, target);
                *target
            }
        };

        tunnel.target = target.to_string();
    }

    Ok(config)
}
