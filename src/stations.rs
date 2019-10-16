use std::fmt;
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
    pub distance_to_arrival: Option<f64>,
    pub market_id: Option<u64>,
    pub name: String,
    #[serde(rename = "type")]
    pub st_type: String,
    #[serde(default)]
    pub system_name: String,
    pub update_time: UpdateTime,
}

impl Station {
    pub fn outdated(&self, now: DateTime<Utc>, days_thres: i64) -> Result<Outdated, Fail> {
        self.update_time.outdated(now, days_thres)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTime {
    information: String,
    market: Option<String>,
    shipyard: Option<String>,
    outfitting: Option<String>,
}

impl UpdateTime {
    fn outdated(&self, now: DateTime<Utc>, days_thres: i64) -> Result<Outdated, Fail> {
        Ok(Outdated {
            information: parse_and_check(now, days_thres, &self.information)?,
            market: map_parse_and_check(now, days_thres, self.market.as_ref())?,
            shipyard: map_parse_and_check(now, days_thres, self.shipyard.as_ref())?,
            outfitting: map_parse_and_check(now, days_thres, self.outfitting.as_ref())?,
        })
    }
}

fn parse_and_check(now: DateTime<Utc>, days_thres: i64, s: &str) -> Result<Option<i64>, Fail> {
    let t = Utc.datetime_from_str(s, "%Y-%m-%d %H:%M:%S")?;
    let days = now.signed_duration_since(t).num_days();
    if days > days_thres {
        Ok(Some(days))
    } else {
        Ok(None)
    }
}

fn map_parse_and_check(
    now: DateTime<Utc>,
    days_thres: i64,
    s: Option<&String>,
) -> Result<Option<i64>, Fail> {
    if let Some(s) = s {
        parse_and_check(now, days_thres, s)
    } else {
        Ok(None)
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
