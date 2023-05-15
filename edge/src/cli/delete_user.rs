use anyhow::{anyhow, Result};
use log::info;

use crate::config::{write_config, Configuration};

pub fn delete_user(cfg: &mut Configuration, name_or_key: String) -> Result<()> {
    let secret = if cfg.secrets.contains_key(&name_or_key.to_lowercase()) {
        name_or_key.to_lowercase()
    } else {
        cfg.secrets
            .iter()
            .find(|(name, _)| **name == name_or_key)
            .map(|(name, _)| name.clone())
            .ok_or_else(|| anyhow!("user not found"))?
    };

    cfg.secrets.remove(&secret);

    write_config(cfg)?;

    info!("user {} deleted", secret);
    Ok(())
}
