use std::env::var;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Deserialize;
use serde_json::from_str;
use tiny_fail::Fail;

use crate::coords::Coords;

pub fn load_current_location() -> Result<Location, Fail> {
    let mut journal_files = journal_files()?;
    let mut buf = String::new();

    while let Some(file_path) = journal_files.pop() {
        let f = File::open(&file_path)?;
        let mut r = BufReader::new(f);
        let mut location = Option::<Location>::None;

        loop {
            r.read_line(&mut buf)?;
            if buf.is_empty() {
                break;
            }

            let event: Event = from_str(&buf).map_err(|e| Fail::new(format!("{}: {}", e, buf)))?;
            if let Some(loc) = event.into_option() {
                location = Some(loc);
            }
            buf.truncate(0);
        }

        if let Some(loc) = location {
            return Ok(loc);
        }
    }

    Err(Fail::new("No location entry"))
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
    #[serde(other)]
    Other,
}

impl Event {
    fn into_option(self) -> Option<Location> {
        match self {
            Event::Location(l) => Some(l),
            Event::FSDJump(l) => Some(l),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Location {
    pub star_system: String,
    pub star_pos: Coords,
}
