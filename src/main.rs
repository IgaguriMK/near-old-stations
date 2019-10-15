mod config;
mod coords;
mod download;
mod journal;
mod stations;

use chrono::Utc;
use regex::RegexSet;
use tiny_fail::{ErrorMessageExt, Fail};

use stations::{load_stations, Station, Outdated};

fn main() {
    if let Err(e) = w_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn w_main() -> Result<(), Fail> {
    let cfg = config::Config::load().err_msg("failed load config")?;

    let exclude_patterns = RegexSet::new(&cfg.excludes).err_msg("failed parse 'exclude'")?;
    let exclude_systems =
        RegexSet::new(&cfg.exclude_systems).err_msg("failed parse 'exclude_systems'")?;

    download::download().err_msg("failed download dump file")?;
    let sts = load_stations().err_msg("failed load dump file")?;

    let (location, visited_stations) =
        journal::load_current_location().err_msg("failed load journals")?;

    let now = Utc::now();
    let mut entries = Vec::<Entry>::new();
    for st in sts.into_iter() {
        let dist = st.coords.dist_to(location.star_pos);
        if dist > cfg.max_dist {
            continue;
        }

        let outdated = st.outdated(now, cfg.days)?;
        if ! outdated.is_outdated() {
            continue;
        }

        if exclude_patterns.is_match(&st.name) {
            continue;
        }
        if exclude_systems.is_match(&st.system_name) {
            continue;
        }

        let visited = st
            .market_id
            .map(|id| visited_stations.contains(&id))
            .unwrap_or(false);
        let distance_to_arrival = st.distance_to_arrival;

        entries.push(Entry {
            st,
            outdated,
            dist,
            distance_to_arrival,
            visited,
        });
    }

    entries.sort_by_key(|entry| entry.score());
    entries.reverse();

    for (i, e) in entries.iter().enumerate() {
        if i == cfg.max_entries {
            break;
        }
        println!(
            "{:<2}{:>6.2} Ly + {:>8} Ls  {}d [{}]  {:<25}  {}",
            if e.visited { "*" } else { " " },
            e.dist,
            si_fmt(e.distance_to_arrival),
            e.outdated.days().unwrap(),
            e.outdated,
            e.st.name,
            e.st.system_name
        );
    }

    Ok(())
}

fn si_fmt(x: Option<f64>) -> String {
    match x {
        None => "unknown".to_owned(),
        Some(x) if x < 100.0 => format!("{:.2}", x),
        Some(x) if x < 1000.0 => format!("{:.1}", x),
        Some(x) if x < 10000.0 => format!("{:.2}k", x / 1000.0),
        Some(x) if x < 100000.0 => format!("{:.1}k", x / 1000.0),
        Some(x) => format!("{:.0}k", x / 1000.0),
    }
}

#[derive(Debug)]
struct Entry {
    st: Station,
    outdated: Outdated,
    dist: f64,
    distance_to_arrival: Option<f64>,
    visited: bool,
}

impl Entry {
    fn score(&self) -> u64 {
        if self.dist < 0.01 {
            std::u64::MAX
        } else {
            ((self.outdated.days().unwrap() as f64) / self.dist) as u64
        }
    }
}
