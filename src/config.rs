use std::fs::File;
use std::io::Read;

use clap::{App, Arg, crate_version};
use serde::Deserialize;
use tiny_fail::{ErrorMessageExt, Fail};
use toml::from_slice;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub max_dist: f64,
    pub days: i64,
    pub max_entries: usize,
    #[serde(default)]
    pub mode: Mode,
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

        let mut cfg: Config = from_slice(&bytes).err_msg("failed parse config")?;

        // args
        let matches = App::new("near-old-stations")
            .version(crate_version!())
            .arg(Arg::with_name("max_dist").long("max-dist").takes_value(true).help("Maximum distance from current position"))
            .arg(Arg::with_name("days").long("days").takes_value(true).help("Minimum days from last update"))
            .arg(Arg::with_name("max_entries").short("n").long("max-entries").takes_value(true).help("Minimum entries to show"))
            .arg(Arg::with_name("mode").long("mode").takes_value(true).possible_values(&["oneshot", "update"]).help("Run mode"))
            .get_matches();

        if let Some(s) = matches.value_of("max_dist") {
            cfg.max_dist = s.parse::<f64>().err_msg("can't parse 'max_dist' as float")?;
        }
        if let Some(s) = matches.value_of("days") {
            cfg.days = s.parse::<i64>().err_msg("can't parse 'days' as int")?;
        }
        if let Some(s) = matches.value_of("max_entries") {
            cfg.max_entries = s.parse::<usize>().err_msg("can't parse 'max_entries' as int")?;
        }
        if let Some(s) = matches.value_of("mode") {
            match s {
                "oneshot" => cfg.mode = Mode::Oneshot,
                "update" => cfg.mode = Mode::Update,
                s => return Err(Fail::new(format!("invalid mode '{}'", s))),
            }
        }

        Ok(cfg)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Oneshot,
    Update,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Oneshot
    }
}
