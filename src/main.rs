mod config;
mod coords;
mod journal;
mod stations;

use chrono::Utc;
use regex::RegexSet;
use tiny_fail::{ErrorMessageExt, Fail};

use stations::{load_stations, Station};

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

    let (location, visited_stations) =
        journal::load_current_location().err_msg("failed load journals")?;
    let sts = load_stations(&cfg.dumps_dir).err_msg("failed load dump file")?;

    let now = Utc::now();
    let mut entries = Vec::<Entry>::new();
    for st in sts.into_iter() {
        let dist = st.coords.dist_to(location.star_pos);
        if dist > cfg.max_dist {
            continue;
        }

        let days = now.signed_duration_since(st.updated_at()?).num_days();
        if days < cfg.days {
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

        entries.push(Entry {
            st,
            days,
            dist,
            visited,
        });
    }

    entries.sort_by_key(|entry| entry.score());

    for (i, e) in entries.iter().enumerate() {
        if i == cfg.max_entries {
            break;
        }
        println!(
            "{:<2}{:.2}\t{}d\t{:<30}\t[{}]",
            if e.visited { "*" } else { " " },
            e.dist,
            e.days,
            e.st.name,
            e.st.system_name
        );
    }

    Ok(())
}

#[derive(Debug)]
struct Entry {
    st: Station,
    days: i64,
    dist: f64,
    visited: bool,
}

impl Entry {
    fn score(&self) -> u64 {
        ((self.days as f64) / self.dist) as u64
    }
}
