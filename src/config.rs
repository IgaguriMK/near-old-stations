use std::collections::HashSet;
use std::fs::File;
use std::io::Read;

use clap::{crate_version, App, Arg};
use regex::RegexSet;
use serde::Deserialize;
use tiny_fail::{ErrorMessageExt, Fail};
use toml::from_slice;

use crate::filter::{Days, Filter, Filters};
use crate::journal::{load_current_location, sol_origin, GetLocFunc};
use crate::mode;
use crate::stations::Economy;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    days: OutdatedDays,
    filter: FilterConfig,
    max_entries: usize,
    #[serde(default)]
    mode: Mode,
    max_dist: f64,
    #[serde(default)]
    pos_origin: Origin,
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

    pub fn filter(&self) -> Result<Filters, Fail> {
        let mut filters = Filters::new();

        filters.add(Filter::Dist(self.max_dist));
        self.days.filter(&mut filters);
        self.filter.filter(&mut filters)?;

        Ok(filters)
    }

    pub fn filter_config(&self) -> &FilterConfig {
        &self.filter
    }

    pub fn get_loc_func(&self) -> GetLocFunc {
        match self.pos_origin {
            Origin::Current => load_current_location,
            Origin::Sol => sol_origin,
        }
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    pub fn mode(&self) -> mode::Mode {
        match self.mode {
            Mode::Oneshot => mode::Mode::Oneshot,
            Mode::Update => mode::Mode::Update,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct OutdatedDays {
    information: Option<i64>,
    market: Option<i64>,
    shipyard: Option<i64>,
    outfitting: Option<i64>,
}

impl OutdatedDays {
    fn filter(&self, filters: &mut Filters) {
        if let Some(days) = self.information {
            filters.add(Filter::Days(Days::Information(days)));
        }
        if let Some(days) = self.market {
            filters.add(Filter::Days(Days::Market(days)));
        }
        if let Some(days) = self.shipyard {
            filters.add(Filter::Days(Days::Shipyard(days)));
        }
        if let Some(days) = self.outfitting {
            filters.add(Filter::Days(Days::Outfitting(days)));
        }
        filters.add(Filter::Outdated);
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

/* Filters */

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FilterConfig {
    #[serde(default)]
    pub exclude_names: Vec<String>,
    #[serde(default)]
    pub exclude_systems: Vec<String>,

    distance_to_arrival: Option<DistanceToArrival>,
    economy: Option<EconomyFilter>,
    pad_size: Option<PadSize>,
    planetary: Option<Planetary>,
}

impl FilterConfig {
    fn filter(&self, filters: &mut Filters) -> Result<(), Fail> {
        filters.add(Filter::StationName(self.exclude_names()?));
        filters.add(Filter::SystemName(self.exclude_systems()?));

        if let Some(ref f) = self.distance_to_arrival {
            f.filter(filters)?;
        }
        if let Some(ref f) = self.economy {
            f.filter(filters)?;
        }
        if let Some(ref f) = self.pad_size {
            f.filter(filters)?;
        }
        if let Some(ref f) = self.planetary {
            f.filter(filters)?;
        }

        Ok(())
    }

    pub fn exclude_names(&self) -> Result<RegexSet, Fail> {
        Ok(RegexSet::new(&self.exclude_names).err_msg("failed parse 'exclude'")?)
    }

    pub fn exclude_systems(&self) -> Result<RegexSet, Fail> {
        Ok(RegexSet::new(&self.exclude_systems).err_msg("failed parse 'exclude_systems'")?)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct DistanceToArrival {
    max: f64,
}

impl DistanceToArrival {
    fn filter(&self, filters: &mut Filters) -> Result<(), Fail> {
        filters.add(Filter::DistToArrival(self.max));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct EconomyFilter {
    list: Vec<Economy>,
    #[serde(default)]
    include_secondary: bool,
}

impl EconomyFilter {
    fn filter(&self, filters: &mut Filters) -> Result<(), Fail> {
        let set: HashSet<Economy> = self.list.iter().cloned().collect();
        filters.add(Filter::Economy(set, self.include_secondary));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct PadSize {
    l_pad_only: bool,
}

impl PadSize {
    fn filter(&self, filters: &mut Filters) -> Result<(), Fail> {
        if self.l_pad_only {
            filters.add(Filter::LPadOnly);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Planetary {
    include: bool,
}

impl Planetary {
    fn filter(&self, filters: &mut Filters) -> Result<(), Fail> {
        if !self.include {
            filters.add(Filter::IgnorePlanetary);
        }
        Ok(())
    }
}
