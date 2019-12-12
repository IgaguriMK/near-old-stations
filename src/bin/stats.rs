use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use chrono::{DateTime, Utc};
use tiny_fail::{ErrorMessageExt, Fail};

use near_old_stations::config::Config;
use near_old_stations::stations::{load_stations, Station};

fn main() {
    if let Err(e) = w_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn w_main() -> Result<(), Fail> {
    let cfg = Config::load().err_msg("failed load config")?;

    let exclude_names = cfg.filter_config().exclude_names()?;
    let exclude_systems = cfg.filter_config().exclude_systems()?;

    let mut sts = Vec::new();
    for st in load_stations()
        .err_msg("failed load dump file")?
        .into_list()
    {
        if exclude_names.is_match(&st.name) {
            continue;
        }
        if exclude_systems.is_match(&st.system_name) {
            continue;
        }

        sts.push(st);
    }

    count(&sts, "days_information.txt", |st| {
        Some(st.update_time().information())
    })?;
    count(&sts, "days_market.txt", |st| st.update_time().market())?;
    count(&sts, "days_shipyard.txt", |st| st.update_time().shipyard())?;
    count(&sts, "days_outfitting.txt", |st| {
        st.update_time().outfitting()
    })?;

    Ok(())
}

fn count(
    sts: &[Station],
    file_name: &str,
    get_val: impl Fn(&Station) -> Option<DateTime<Utc>>,
) -> Result<(), Fail> {
    let mut cnt = BTreeMap::<i64, usize>::new();

    let now = Utc::now();
    for st in sts {
        if let Some(t) = get_val(st) {
            let d = now.signed_duration_since(t).num_days();
            cnt.entry(d).and_modify(|c| *c += 1).or_insert(1);
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
