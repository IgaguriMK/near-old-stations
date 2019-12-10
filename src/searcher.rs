use chrono::Utc;

use crate::journal::{Location, Visited};
use crate::stations::{Station, Stations};

pub struct Searcher<F> {
    stations: Stations,
    filter: F,
}

impl<F: Filter> Searcher<F> {
    pub fn new(stations: Stations, filter: F) -> Searcher<F> {
        Searcher { stations, filter }
    }

    pub fn search(&self, loc: &Location, visited: &Visited) -> Vec<Record> {
        let now = Utc::now();

        let mut records = Vec::new();
        for station in self.stations.stations() {
            let distance = loc.star_pos.dist_to(station.coords);
            let visited = station
                .market_id
                .map(|id| visited.is_visited(id))
                .unwrap_or(false);

            let update_time = station.update_time();
            let information_days = Days::new(
                now.signed_duration_since(update_time.information())
                    .num_days(),
            );
            let market_days = if let Some(t) = update_time.market() {
                Days::new(now.signed_duration_since(t).num_days())
            } else {
                Days::empty()
            };
            let shipyard_days = if let Some(t) = update_time.shipyard() {
                Days::new(now.signed_duration_since(t).num_days())
            } else {
                Days::empty()
            };
            let outfitting_days = if let Some(t) = update_time.outfitting() {
                Days::new(now.signed_duration_since(t).num_days())
            } else {
                Days::empty()
            };

            let mut record = Record {
                station,
                distance,
                visited,
                information_days,
                market_days,
                shipyard_days,
                outfitting_days,
            };

            if self.filter.filter(&mut record) {
                records.push(record);
            }
        }

        records.sort_by(|l, r| l.cmp(r).reverse());
        records
    }
}

pub trait Filter {
    fn filter(&self, record: &mut Record) -> bool;
}

#[derive(Debug)]
pub struct Record<'a> {
    pub station: &'a Station,
    pub distance: f64,
    pub visited: bool,
    pub information_days: Days,
    pub market_days: Days,
    pub shipyard_days: Days,
    pub outfitting_days: Days,
}

impl<'a> Record<'a> {
    fn score(&self) -> f64 {
        if let Some(days) = self.outdated() {
            let dist =
                self.distance + 0.000_000_1 * self.station.distance_to_arrival.unwrap_or(0.0);
            (days as f64) / dist
        } else {
            0.0
        }
    }

    pub fn outdated(&self) -> Option<i64> {
        let mut max = i64::min_value();

        if let Some(v) = self.information_days.outdated() {
            max = max.max(v);
        }
        if let Some(v) = self.market_days.outdated() {
            max = max.max(v);
        }
        if let Some(v) = self.shipyard_days.outdated() {
            max = max.max(v);
        }
        if let Some(v) = self.outfitting_days.outdated() {
            max = max.max(v);
        }

        if max > i64::min_value() {
            Some(max)
        } else {
            None
        }
    }
}

impl<'a> PartialEq for Record<'a> {
    fn eq(&self, other: &Record) -> bool {
        self.score() == other.score()
    }
}

impl<'a> Eq for Record<'a> {}

impl<'a> PartialOrd for Record<'a> {
    fn partial_cmp(&self, other: &Record) -> Option<std::cmp::Ordering> {
        self.score().partial_cmp(&other.score())
    }
}

impl<'a> Ord for Record<'a> {
    fn cmp(&self, other: &Record) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug)]
pub struct Days {
    days: Option<i64>,
    outdated: Option<i64>,
}

impl Days {
    fn new(days: i64) -> Days {
        Days {
            days: Some(days),
            outdated: None,
        }
    }

    fn empty() -> Days {
        Days {
            days: None,
            outdated: None,
        }
    }

    pub fn check(&mut self, check_outdated: impl FnOnce(i64) -> bool) {
        if let Some(days) = self.days {
            if check_outdated(days) {
                self.outdated = Some(days);
            }
        }
    }

    fn outdated(&self) -> Option<i64> {
        self.outdated
    }

    pub fn is_outdated(&self) -> bool {
        self.outdated.is_some()
    }
}
