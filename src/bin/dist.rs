use std::fs::File;
use std::io::{BufWriter, Write};

use chrono::Utc;
use regex::RegexSet;
use tiny_fail::{ErrorMessageExt, Fail};

use near_old_stations::config::Config;
use near_old_stations::download::download;
use near_old_stations::stations::load_stations;

fn main() {
    if let Err(e) = w_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn w_main() -> Result<(), Fail> {
    let cfg = Config::load().err_msg("failed load config")?;

    let exclude_patterns = RegexSet::new(&cfg.excludes).err_msg("failed parse 'exclude'")?;
    let exclude_systems =
        RegexSet::new(&cfg.exclude_systems).err_msg("failed parse 'exclude_systems'")?;

    let now = Utc::now();

    download().err_msg("failed download dump file")?;
    let sts = load_stations().err_msg("failed load dump file")?;

    let mut information_file = BufWriter::new(File::create("days_information.txt")?);
    let mut market_file = BufWriter::new(File::create("days_market.txt")?);
    let mut shipyard_file = BufWriter::new(File::create("days_shipyard.txt")?);
    let mut outfitting_file = BufWriter::new(File::create("days_outfitting.txt")?);

    for st in &sts {
        if exclude_patterns.is_match(&st.name) {
            continue;
        }
        if exclude_systems.is_match(&st.system_name) {
            continue;
        }

        let days = st.outdated(now, -1)?;

        if let Some(d) = days.information {
            writeln!(&mut information_file, "{}", d)?;
        }
        if let Some(d) = days.market {
            writeln!(&mut market_file, "{}", d)?;
        }
        if let Some(d) = days.shipyard {
            writeln!(&mut shipyard_file, "{}", d)?;
        }
        if let Some(d) = days.outfitting {
            writeln!(&mut outfitting_file, "{}", d)?;
        }
    }

    Ok(())
}
