use std::fs::File;
use std::io::{BufRead, BufReader};

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::from_str;
use tiny_fail::Fail;

use crate::coords::Coords;

pub fn load_stations() -> Result<Vec<Station>, Fail> {
    let f = File::open("./systemsPopulated.json")?;
    let mut r = BufReader::new(f);

    let mut list = Vec::new();
    let mut buf = String::new();
    loop {
        r.read_line(&mut buf)?;
        let s = buf.trim().trim_end_matches(',');
        if s == "[" {
            buf.truncate(0);
            continue;
        }
        if s == "]" {
            break;
        }

        let sys: System = from_str(s).map_err(|e| Fail::new(format!("{}: {}", e, s)))?;
        buf.truncate(0);
        for st in &sys.stations {
            let mut st = st.clone();
            st.coords = sys.coords;
            st.system_name = sys.name.clone();
            list.push(st);
        }
    }

    Ok(list)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    name: String,
    coords: Coords,
    stations: Vec<Station>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    #[serde(default)]
    pub coords: Coords,
    #[serde(default)]
    pub system_name: String,
    pub name: String,
    #[serde(rename = "type")]
    pub st_type: String,
    pub market_id: Option<u64>,
    pub update_time: UpdateTime,
}

impl Station {
    pub fn updated_at(&self) -> Result<DateTime<Utc>, Fail> {
        self.update_time.updated_at()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTime {
    information: String,
}

impl UpdateTime {
    fn updated_at(&self) -> Result<DateTime<Utc>, Fail> {
        Ok(Utc.datetime_from_str(&self.information, "%Y-%m-%d %H:%M:%S")?)
    }
}
