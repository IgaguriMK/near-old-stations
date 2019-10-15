use std::fs::File;
use std::io::Read;

use serde::Deserialize;
use tiny_fail::{ErrorMessageExt, Fail};
use toml::from_slice;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub max_dist: f64,
    pub days: i64,
    pub max_entries: usize,
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default)]
    pub exclude_systems: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Config, Fail> {
        let mut f = File::open("./config.toml").err_msg("failed open config file")?;

        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)
            .err_msg("failed read config file")?;

        let cfg = from_slice(&bytes).err_msg("failed parse config")?;
        Ok(cfg)
    }
}
