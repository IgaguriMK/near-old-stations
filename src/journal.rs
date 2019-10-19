use std::collections::HashSet;
use std::env::var;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Deserialize;
use serde_json::from_str;
use tiny_fail::Fail;

use crate::coords::Coords;

const VISITED_VIEW_FILES: usize = 50;

pub fn sol_origin() -> Result<(Location, HashSet<u64>), Fail> {
    let (_, visited) = load_current_location()?;

    Ok((
        Location {
            star_system: "Sol".to_owned(),
            star_pos: Coords::zero(),
        },
        visited,
    ))
}

pub fn load_current_location() -> Result<(Location, HashSet<u64>), Fail> {
    let mut journal_files = journal_files()?;
    let mut buf = String::new();

    let mut location = Option::<Location>::None;
    let mut visited_stations = HashSet::<u64>::new();

    while let Some(file_path) = journal_files.pop() {
        let f = File::open(&file_path)?;
        let mut r = BufReader::new(f);

        loop {
            r.read_line(&mut buf)?;
            if buf.is_empty() {
                break;
            }

            let event: Event = from_str(&buf).map_err(|e| Fail::new(format!("{}: {}", e, buf)))?;
            buf.truncate(0);
            match event {
                Event::Location(loc) => location = Some(loc),
                Event::FSDJump(loc) => location = Some(loc),
                Event::Docked(docked) => {
                    visited_stations.insert(docked.market_id);
                }
                _ => {}
            }
        }

        if location.is_some() {
            break;
        }
    }

    let mut cnt = VISITED_VIEW_FILES;
    while let Some(file_path) = journal_files.pop() {
        if cnt == 0 {
            break;
        }
        cnt -= 1;

        let f = File::open(&file_path)?;
        let mut r = BufReader::new(f);

        loop {
            r.read_line(&mut buf)?;
            if buf.is_empty() {
                break;
            }

            let event: Event = from_str(&buf).map_err(|e| Fail::new(format!("{}: {}", e, buf)))?;
            buf.truncate(0);
            match event {
                Event::Docked(docked) => {
                    visited_stations.insert(docked.market_id);
                }
                _ => {}
            }
        }
    }

    if let Some(loc) = location {
        Ok((loc, visited_stations))
    } else {
        Err(Fail::new("No location entry"))
    }
}

fn journal_files() -> Result<Vec<PathBuf>, Fail> {
    let journal_dir = journal_dir()?;
    let journal_regex = Regex::new(r"^Journal\.\d{12}\.\d{2}\.log$")?;

    let journal_files = journal_dir
        .read_dir()?
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|p| {
            if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                return journal_regex.is_match(n);
            }
            false
        })
        .collect();

    Ok(journal_files)
}

fn journal_dir() -> Result<PathBuf, Fail> {
    let home = var("USERPROFILE")?;
    let journal_dir = Path::new(&home).join(r"Saved Games\Frontier Developments\Elite Dangerous");
    if !journal_dir.exists() {
        return Err(Fail::new(format!(
            "'{}' is not exists.",
            journal_dir.to_string_lossy()
        )));
    }
    if !journal_dir.is_dir() {
        return Err(Fail::new(format!(
            "'{}' is not dir.",
            journal_dir.to_string_lossy()
        )));
    }
    Ok(journal_dir)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "event")]
enum Event {
    Location(Location),
    FSDJump(Location),
    Docked(Docked),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Location {
    pub star_system: String,
    pub star_pos: Coords,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Docked {
    #[serde(rename = "MarketID")]
    market_id: u64,
    timestamp: String,
}
