use anyhow::{bail, Result};
use log::info;
use rand::random;

use crate::config::{write_config, Configuration, Secret};

pub fn add_user(cfg: &mut Configuration, name: String, max_tunnels: Option<usize>) -> Result<()> {
    if cfg.secrets.contains_key(&name.to_lowercase()) {
        bail!("user already exists");
    }

    let max_tunnels = max_tunnels.unwrap_or(5);
    let key = generate_key();

    cfg.secrets
        .insert(name.to_lowercase(), Secret { max_tunnels, key });

    write_config(cfg)?;

    info!("user {} added", name);
    Ok(())
}

fn generate_key() -> String {
    let bytes = random::<[u8; 32]>();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
