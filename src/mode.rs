use std::thread::sleep;
use std::time::{Duration, Instant};

use chrono::Utc;
use tiny_fail::{ErrorMessageExt, Fail};

use crate::journal::GetLocFunc;
use crate::printer::Printer;
use crate::searcher::{Filter, Searcher};
use crate::stations::Stations;

const UPDATE_POOL_PERIOD: Duration = Duration::from_secs(5);
const FORCE_UPDATE_PERIOD: Duration = Duration::from_secs(60);

pub enum Mode {
    Oneshot,
    Update,
}

impl Mode {
    pub fn run(
        &self,
        stations: Stations,
        get_loc_func: GetLocFunc,
        filter: impl Filter,
        mut printer: impl Printer,
        max_entries: usize,
    ) -> Result<(), Fail> {
        let last_mod = stations
            .last_mod()
            .err_msg("No stations update date info.")?
            .with_timezone(&Utc);

        let searcher = Searcher::new(stations, filter);

        match self {
            Mode::Oneshot => {
                let (location, visited) = get_loc_func()?;
                let records = searcher.search(&location, &visited);
                printer.print(&records, max_entries, last_mod)?;
                Ok(())
            }
            Mode::Update => {
                let (location, visited) = get_loc_func()?;
                let records = searcher.search(&location, &visited);
                printer.print(&records, max_entries, last_mod)?;

                let mut prev_location = location;
                let mut prev_visited = visited;
                let mut last_update = Instant::now();

                loop {
                    sleep(UPDATE_POOL_PERIOD);

                    let (location, visited) = get_loc_func()?;
                    if location == prev_location
                        && visited == prev_visited
                        && last_update.elapsed() < FORCE_UPDATE_PERIOD
                    {
                        continue;
                    }

                    let records = searcher.search(&location, &visited);
                    printer.clear()?;
                    printer.print(&records, max_entries, last_mod)?;

                    prev_location = location;
                    prev_visited = visited;
                    last_update = Instant::now();
                }
            }
        }
    }
}
