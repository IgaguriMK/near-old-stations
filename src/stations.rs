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

const SYTEMS_DUMP_URL: &str = "https://www.edsm.net/dump/systemsPopulated.json.gz";
const SYTEMS_DUMP_FILE: &str = "systemsPopulated.json.gz";
const SYTEMS_COORDS_FILE: &str = "coordinates.json.gz";
const STATIONS_DUMP_URL: &str = "https://www.edsm.net/dump/stations.json.gz";
const STATIONS_DUMP_FILE: &str = "stations.json.gz";

pub fn load_stations() -> Result<Stations, Fail> {
    let downloader = Downloader::new()?;

    let stations = load_raw_stations(&downloader)?;
    let coords_table = load_coords(&downloader, false)?;

    let last_mod = stations.last_mod();
    let mut list = Vec::new();
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
    pub fn stations(&self) -> impl Iterator<Item = &Station> {
        self.list.iter()
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
    pub economy: Option<Economy>,
    pub market_id: Option<u64>,
    pub name: String,
    pub second_economy: Option<Economy>,
    #[serde(rename = "type")]
    pub st_type: StationType,
    pub system_id: u64,
    pub system_name: String,
    pub update_time: UpdateTime,
}

impl Station {
    pub fn update_time(&self) -> &UpdateTime {
        &self.update_time
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
    pub fn information(&self) -> DateTime<Utc> {
        self.information
    }

    pub fn market(&self) -> Option<DateTime<Utc>> {
        self.market
    }

    pub fn shipyard(&self) -> Option<DateTime<Utc>> {
        self.shipyard
    }

    pub fn outfitting(&self) -> Option<DateTime<Utc>> {
        self.outfitting
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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

impl StationType {
    pub fn has_l_pad(self) -> bool {
        match self {
            StationType::Outpost => false,
            _ => true,
        }
    }

    pub fn is_planetary(self) -> bool {
        match self {
            StationType::PlanetaryPort => true,
            StationType::PlanetaryOutpost => true,
            _ => false,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Economy {
    Agriculture,
    Colony,
    Extraction,
    #[serde(rename = "High Tech")]
    HighTech,
    Industrial,
    Military,
    Prison,
    Refinery,
    Repair,
    Rescue,
    Service,
    Terraforming,
    Tourism,
}
