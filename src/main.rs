use std::thread::sleep;
use std::time::Duration;

use chrono::Utc;
use regex::RegexSet;
use tiny_fail::{ErrorMessageExt, Fail};

use near_old_stations::config::{Config, Mode, Origin};
use near_old_stations::download::download;
use near_old_stations::journal::{load_current_location, sol_origin};
use near_old_stations::stations::{load_stations, Outdated, Station};

const UPDATE_INTERVAL: u64 = 5;

fn main() {
    if let Err(e) = w_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn w_main() -> Result<(), Fail> {
    let cfg = Config::load().err_msg("failed load config")?;

    let get_loc_func = match cfg.pos_origin {
        Origin::Current => load_current_location,
        Origin::Sol => sol_origin,
    };

    let exclude_patterns = RegexSet::new(&cfg.excludes).err_msg("failed parse 'exclude'")?;
    let exclude_systems =
        RegexSet::new(&cfg.exclude_systems).err_msg("failed parse 'exclude_systems'")?;

    download().err_msg("failed download dump file")?;
    let sts = load_stations().err_msg("failed load dump file")?;

    let mut last_location = None;

    loop {
        let (location, visited_stations) = get_loc_func().err_msg("failed load journals")?;

        if let Some((ref last_loc, docked_cnt)) = last_location {
            if last_loc == &location && docked_cnt == visited_stations.len() {
                sleep(Duration::from_secs(10));
                continue;
            }
        }

        let now = Utc::now();
        let mut entries = Vec::<Entry>::new();
        for st in &sts {
            let dist = st.coords.dist_to(location.star_pos);
            if dist > cfg.max_dist {
                continue;
            }

            let outdated = st.outdated(now, cfg.days)?;
            if !outdated.is_outdated() {
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

        if cfg.mode == Mode::Update {
            println!("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n");
        }
        println!("Total: {} stations.", entries.len());
        for (i, e) in entries.iter().enumerate() {
            if i == cfg.max_entries {
                break;
            }
            println!(
                "{:>3}{:<2}{:>6.2} Ly + {:>8} Ls  {}d [{}]  {:<25} {:<12} ({})",
                i + 1,
                if e.visited { "*" } else { " " },
                e.dist,
                si_fmt(e.distance_to_arrival),
                e.outdated.days().unwrap(),
                e.outdated,
                e.st.name,
                e.st.system_name,
                e.st.st_type,
            );
        }

        match cfg.mode {
            Mode::Oneshot => return Ok(()),
            Mode::Update => {
                last_location = Some((location, visited_stations.len()));
                sleep(Duration::from_secs(UPDATE_INTERVAL));
            }
        }
    }
}

fn si_fmt(x: Option<f64>) -> String {
    match x {
        None => "unknown".to_owned(),
        Some(x) if x < 100.0 => format!("{:.2} ", x),
        Some(x) if x < 1000.0 => format!("{:.1} ", x),
        Some(x) if x < 10000.0 => format!("{:.2}k", x / 1000.0),
        Some(x) if x < 100000.0 => format!("{:.1}k", x / 1000.0),
        Some(x) => format!("{:.0}k", x / 1000.0),
    }
}

#[derive(Debug)]
struct Entry<'a> {
    st: &'a Station,
    outdated: Outdated,
    dist: f64,
    distance_to_arrival: Option<f64>,
    visited: bool,
}

impl<'a> Entry<'a> {
    fn score(&self) -> Score {
        if self.dist < 0.01 {
            Score::new(std::u64::MAX, self.st.distance_to_arrival)
        } else {
            Score::new(
                ((self.outdated.days().unwrap() as f64) / self.dist) as u64,
                self.st.distance_to_arrival,
            )
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    sys: u64,
    arrival: i64,
}

impl Score {
    fn new(sys: u64, arrival: Option<f64>) -> Score {
        Score {
            sys,
            arrival: arrival
                .map(|a| (a * -100000.0) as i64)
                .unwrap_or(std::i64::MIN),
        }
    }
}
