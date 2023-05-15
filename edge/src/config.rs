use std::{collections::HashMap, fs::read_to_string};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use toml::from_str;

const CONFIG_PATH: &str = if cfg!(debug_assertions) {
    "edge/config.toml"
} else {
    "config.toml"
};

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub port: u16,
    pub secrets: HashMap<String, Secret>,
}

#[derive(Deserialize, Serialize)]
pub struct Secret {
    pub max_tunnels: usize,
    pub key: String,
}

pub fn load_config() -> Result<Configuration> {
    let file = read_to_string(CONFIG_PATH)?;
    let config: Configuration = from_str(&file)?;
    Ok(config)
}

pub fn write_config(config: &Configuration) -> Result<()> {
    let file = toml::to_string(&config)?;
    std::fs::write(CONFIG_PATH, file)?;
    Ok(())
}
