use std::fs::File;
use std::io::Read;

use clap::{crate_version, App, Arg};
use serde::Deserialize;
use tiny_fail::{ErrorMessageExt, Fail};
use toml::from_slice;

use crate::stations::Criteria;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub max_dist: f64,
    pub days: i64,
    pub max_entries: usize,
    #[serde(default = "default_true")]
    pub information: bool,
    #[serde(default = "default_true")]
    pub market: bool,
    #[serde(default = "default_true")]
    pub shipyard: bool,
    #[serde(default = "default_true")]
    pub outfitting: bool,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub pos_origin: Origin,
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
            .arg(
                Arg::with_name("max_dist")
                    .long("max-dist")
                    .takes_value(true)
                    .help("Maximum distance from current position"),
            )
            .arg(
                Arg::with_name("days")
                    .long("days")
                    .takes_value(true)
                    .help("Minimum days from last update"),
            )
            .arg(
                Arg::with_name("max_entries")
                    .short("n")
                    .long("max-entries")
                    .takes_value(true)
                    .help("Minimum entries to show"),
            )
            .arg(
                Arg::with_name("mode")
                    .long("mode")
                    .takes_value(true)
                    .possible_values(&["oneshot", "update"])
                    .help("Run mode"),
            )
            .arg(
                Arg::with_name("pos_origin")
                    .long("pos-origin")
                    .takes_value(true)
                    .possible_values(&["current", "Sol"])
                    .help("Disctance calculation origin"),
            )
            .get_matches();

        if let Some(s) = matches.value_of("max_dist") {
            cfg.max_dist = s
                .parse::<f64>()
                .err_msg("can't parse 'max_dist' as float")?;
        }

        if let Some(s) = matches.value_of("days") {
            cfg.days = s.parse::<i64>().err_msg("can't parse 'days' as int")?;
        }

        if let Some(s) = matches.value_of("max_entries") {
            cfg.max_entries = s
                .parse::<usize>()
                .err_msg("can't parse 'max_entries' as int")?;
        }

        if let Some(s) = matches.value_of("mode") {
            match s {
                "oneshot" => cfg.mode = Mode::Oneshot,
                "update" => cfg.mode = Mode::Update,
                s => unreachable!("unreachable branch of match 'mode' with {}", s),
            }
        }
        if let Some(s) = matches.value_of("pos_origin") {
            match s {
                "current" => cfg.pos_origin = Origin::Current,
                "Sol" => cfg.pos_origin = Origin::Sol,
                s => unreachable!("unreachable branch of match 'pos_origin' with {}", s),
            }
        }

        Ok(cfg)
    }
}

impl Criteria for Config {
    fn days(&self) -> i64 {
        self.days
    }
    fn information(&self) -> bool {
        self.information
    }
    fn market(&self) -> bool {
        self.market
    }
    fn shipyard(&self) -> bool {
        self.shipyard
    }
    fn outfitting(&self) -> bool {
        self.outfitting
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum Origin {
    #[serde(rename = "current")]
    Current,
    Sol,
}

impl Default for Origin {
    fn default() -> Origin {
        Origin::Current
    }
}

fn default_true() -> bool {
    true
}
