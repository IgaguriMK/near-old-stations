use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use chrono::Utc;
use regex::RegexSet;
use tiny_fail::{ErrorMessageExt, Fail};

use near_old_stations::config::Config;
use near_old_stations::download::download;
use near_old_stations::stations::{load_stations, Outdated, Station};

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

    download().err_msg("failed download dump file")?;

    let mut sts = Vec::new();
    for st in load_stations().err_msg("failed load dump file")? {
        if exclude_patterns.is_match(&st.name) {
            continue;
        }
        if exclude_systems.is_match(&st.system_name) {
            continue;
        }

        sts.push(st);
    }

    count(&sts, "days_information.txt", |st| st.information)?;
    count(&sts, "days_market.txt", |st| st.market)?;
    count(&sts, "days_shipyard.txt", |st| st.shipyard)?;
    count(&sts, "days_outfitting.txt", |st| st.outfitting)?;

    Ok(())
}

fn count(
    sts: &[Station],
    file_name: &str,
    get_val: impl Fn(Outdated) -> Option<i64>,
) -> Result<(), Fail> {
    let mut cnt = BTreeMap::<i64, usize>::new();

    let now = Utc::now();
    for st in sts {
        let days = st.outdated(now, -1)?;
        if let Some(v) = get_val(days) {
            cnt.entry(v).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    let mut w = BufWriter::new(File::create(file_name)?);
    writeln!(w, "Day\tCount\tAcc")?;
    let mut acc = 0usize;
    for (&d, &c) in cnt.iter() {
        acc += c;
        writeln!(w, "{}\t{}\t{}", d, c, acc)?;
    }

    Ok(())
}
