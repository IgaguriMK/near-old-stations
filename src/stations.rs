pub mod download;

mod date_format;
mod date_format_opt;

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use chrono::{DateTime, FixedOffset, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, from_str, to_writer};
use tiny_fail::{ErrorMessageExt, Fail};

use crate::coords::Coords;
use download::Downloader;

const SYTEMS_DUMP_URL: &str = "https://www.edsm.net/dump/systemsPopulated.json";
const SYTEMS_DUMP_FILE: &str = "systemsPopulated.json.gz";
const SYTEMS_COORDS_FILE: &str = "coordinates.json.gz";
const STATIONS_DUMP_URL: &str = "https://www.edsm.net/dump/stations.json";
const STATIONS_DUMP_FILE: &str = "stations.json.gz";

pub fn load_stations() -> Result<Stations, Fail> {
    let downloader = Downloader::new()?;

    let stations = load_raw_stations(&downloader)?;
    let coords_table = load_coords(&downloader, false)?;

    let last_mod = stations.last_mod();
    let mut list = Vec::with_capacity(stations.stations().len());
    let mut missing_coords_stations = Vec::new();
    for mut st in stations.into_list() {
        if let Some(&c) = coords_table.get(&st.system_id) {
            st.coords = c;
            list.push(st);
        } else {
            missing_coords_stations.push(st);
        }
    }

    Ok(Stations {
        list,
        last_mod,
        missing_coords_stations,
    })
}

fn load_raw_stations(downloader: &Downloader) -> Result<Stations, Fail> {
    let last_mod = downloader
        .download(STATIONS_DUMP_FILE, STATIONS_DUMP_URL)
        .err_msg("failed to download stations dump file")?;

    let mut decoder = Decoder::open(STATIONS_DUMP_FILE)?;

    let mut list = Vec::new();
    while let Some(st) = decoder.next::<Station>()? {
        list.push(st);
    }

    Ok(Stations {
        list,
        last_mod,
        missing_coords_stations: Vec::new(),
    })
}

fn load_coords(downloader: &Downloader, force_update: bool) -> Result<HashMap<u64, Coords>, Fail> {
    let coords_file_path = Path::new(SYTEMS_COORDS_FILE);

    // Update coords file.
    if force_update || !coords_file_path.exists() {
        update_coords(downloader)?;
    }

    let f = File::open(coords_file_path).err_msg("can't open coordinates file")?;
    let r = GzDecoder::new(f);
    let list: Vec<System> = from_reader(r).err_msg("failed to decode coordinates")?;

    let mut table = HashMap::new();
    for sys in list {
        table.insert(sys.id, sys.coords);
    }

    Ok(table)
}

fn update_coords(downloader: &Downloader) -> Result<(), Fail> {
    downloader
        .download(SYTEMS_DUMP_FILE, SYTEMS_DUMP_URL)
        .err_msg("failed to download systemsPopulated dump file")?;

    let mut decoder = Decoder::open(SYTEMS_DUMP_FILE)?;
    let mut list = Vec::new();
    while let Some(sys) = decoder.next::<System>()? {
        list.push(sys);
    }

    let f = File::create(SYTEMS_COORDS_FILE).err_msg("failed to create coordinates file")?;
    let w = GzEncoder::new(f, Compression::best());
    to_writer(w, &list).err_msg("failed to encode coordinates")?;

    Ok(())
}

struct Decoder<R: BufRead> {
    r: R,
    buf: String,
}

impl Decoder<BufReader<GzDecoder<File>>> {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Decoder<BufReader<GzDecoder<File>>>, Fail> {
        let f = File::open(&path)
            .err_msg(format!("failed to open file {:?} to decode", path.as_ref()))?;
        let r = BufReader::new(GzDecoder::new(f));
        Ok(Decoder::new(r))
    }
}

impl<R: BufRead> Decoder<R> {
    pub fn new(r: R) -> Decoder<R> {
        Decoder {
            r,
            buf: String::new(),
        }
    }

    pub fn next<D: DeserializeOwned>(&mut self) -> Result<Option<D>, Fail> {
        loop {
            self.r.read_line(&mut self.buf)?;
            let s = self.buf.trim().trim_end_matches(',');
            if s == "[" {
                self.buf.truncate(0);
                continue;
            }
            if s == "]" {
                return Ok(None);
            }

            let item: D = from_str(s).map_err(|e| Fail::new(format!("{}: {}", e, s)))?;
            self.buf.truncate(0);

            return Ok(Some(item));
        }
    }
}

#[derive(Debug)]
pub struct Stations {
    list: Vec<Station>,
    missing_coords_stations: Vec<Station>,
    last_mod: Option<DateTime<FixedOffset>>,
}

impl Stations {
    pub fn stations(&self) -> &[Station] {
        &self.list
    }

    pub fn into_list(self) -> Vec<Station> {
        self.list
    }

    pub fn last_mod(&self) -> Option<DateTime<FixedOffset>> {
        self.last_mod
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    id: u64,
    coords: Coords,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    #[serde(default)]
    pub coords: Coords,
    pub distance_to_arrival: Option<f64>,
    pub market_id: Option<u64>,
    pub name: String,
    #[serde(rename = "type")]
    pub st_type: StationType,
    pub system_id: u64,
    pub system_name: String,
    pub update_time: UpdateTime,
}

impl Station {
    pub fn outdated(&self, now: DateTime<Utc>, criteria: impl Criteria) -> Outdated {
        self.update_time.outdated(now, criteria)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTime {
    #[serde(with = "date_format")]
    information: DateTime<Utc>,
    #[serde(with = "date_format_opt")]
    market: Option<DateTime<Utc>>,
    #[serde(with = "date_format_opt")]
    shipyard: Option<DateTime<Utc>>,
    #[serde(with = "date_format_opt")]
    outfitting: Option<DateTime<Utc>>,
}

impl UpdateTime {
    fn outdated(&self, now: DateTime<Utc>, criteria: impl Criteria) -> Outdated {
        let information = if criteria.information() {
            check(self.information, now, criteria.days())
        } else {
            None
        };

        let market = if criteria.market() {
            flatten(self.market.map(|t| check(t, now, criteria.days())))
        } else {
            None
        };

        let shipyard = if criteria.shipyard() {
            flatten(self.shipyard.map(|t| check(t, now, criteria.days())))
        } else {
            None
        };

        let outfitting = if criteria.outfitting() {
            flatten(self.outfitting.map(|t| check(t, now, criteria.days())))
        } else {
            None
        };

        Outdated {
            information,
            market,
            shipyard,
            outfitting,
        }
    }
}

fn check(t: DateTime<Utc>, now: DateTime<Utc>, days_thres: i64) -> Option<i64> {
    let days = now.signed_duration_since(t).num_days();
    if days > days_thres {
        Some(days)
    } else {
        None
    }
}

fn flatten<T>(opt: Option<Option<T>>) -> Option<T> {
    match opt {
        Some(Some(x)) => Some(x),
        _ => None,
    }
}

pub trait Criteria {
    fn days(&self) -> i64;
    fn information(&self) -> bool;
    fn market(&self) -> bool;
    fn shipyard(&self) -> bool;
    fn outfitting(&self) -> bool;
}

impl <C: Criteria> Criteria for &C {
    fn days(&self) -> i64 {
        (*self).days()
    }
    fn information(&self) -> bool {
        (*self).information()
    }
    fn market(&self) -> bool {
        (*self).market()
    }
    fn shipyard(&self) -> bool {
        (*self).shipyard()
    }
    fn outfitting(&self) -> bool {
        (*self).outfitting()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct All;

impl Criteria for All {
    fn days(&self) -> i64 {
        -1
    }
    fn information(&self) -> bool {
        true
    }
    fn market(&self) -> bool {
        true
    }
    fn shipyard(&self) -> bool {
        true
    }
    fn outfitting(&self) -> bool {
        true
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Outdated {
    pub information: Option<i64>,
    pub market: Option<i64>,
    pub shipyard: Option<i64>,
    pub outfitting: Option<i64>,
}

impl Outdated {
    pub fn is_outdated(self) -> bool {
        self.information.is_some()
            || self.market.is_some()
            || self.shipyard.is_some()
            || self.outfitting.is_some()
    }

    pub fn days(self) -> Option<i64> {
        let mut res = self.information;

        if self.market > res {
            res = self.market;
        }
        if self.shipyard > res {
            res = self.shipyard;
        }
        if self.outfitting > res {
            res = self.outfitting;
        }

        res
    }
}

impl fmt::Display for Outdated {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.information.is_some() {
            write!(f, "I")?;
        } else {
            write!(f, " ")?;
        }

        if self.market.is_some() {
            write!(f, "M")?;
        } else {
            write!(f, " ")?;
        }

        if self.shipyard.is_some() {
            write!(f, "S")?;
        } else {
            write!(f, " ")?;
        }
        if self.outfitting.is_some() {
            write!(f, "O")?;
        } else {
            write!(f, " ")?;
        }

        Ok(())
    }
}

#[test]
fn outdated_days() {
    let outdated = Outdated {
        information: Some(1),
        market: Some(2),
        shipyard: Some(3),
        outfitting: Some(4),
    };

    assert_eq!(outdated.days(), Some(4));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum StationType {
    // Orbital Large
    #[serde(rename = "Ocellus Starport")]
    OcellusStarport,
    #[serde(rename = "Orbis Starport")]
    OrbisStarport,
    #[serde(rename = "Coriolis Starport")]
    CoriolisStarport,
    #[serde(rename = "Asteroid base")]
    AsteroidBase,
    #[serde(rename = "Mega ship")]
    MegaShip,
    // Orbital small
    Outpost,
    // Planetary
    #[serde(rename = "Planetary Port")]
    PlanetaryPort,
    #[serde(rename = "Planetary Outpost")]
    PlanetaryOutpost,
}

impl fmt::Display for StationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StationType::OcellusStarport => write!(f, "Ocellus"),
            StationType::OrbisStarport => write!(f, "Orbis"),
            StationType::CoriolisStarport => write!(f, "Coriolis"),
            StationType::AsteroidBase => write!(f, "Asteroid"),
            StationType::MegaShip => write!(f, "MegaShip"),
            StationType::Outpost => write!(f, "Outpost"),
            StationType::PlanetaryPort => write!(f, "PlanetaryPort"),
            StationType::PlanetaryOutpost => write!(f, "PlanetaryOutpost"),
        }
    }
}
