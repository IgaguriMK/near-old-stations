use std::collections::HashSet;

use regex::RegexSet;

use crate::searcher::{self, Record};
use crate::stations::Economy;

#[derive(Debug, Default, Clone)]
pub struct Filters(Vec<Filter>);

impl Filters {
    pub fn new() -> Filters {
        Filters(Vec::new())
    }

    pub fn add(&mut self, filter: Filter) {
        self.0.push(filter);
    }
}

impl searcher::Filter for Filters {
    fn filter<'a>(&self, record: &mut Record<'a>) -> bool {
        for f in &self.0 {
            if !f.filter(record) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub enum Filter {
    Days(Days),
    Dist(f64),
    DistToArrival(f64),
    Economy(HashSet<Economy>, bool),
    IgnorePlanetary,
    LPadOnly,
    Outdated,
    StationName(RegexSet),
    SystemName(RegexSet),
}

impl searcher::Filter for Filter {
    fn filter<'a>(&self, record: &mut Record<'a>) -> bool {
        match self {
            Filter::Days(days) => days.filter(record),
            Filter::Dist(dist) => record.distance <= *dist,
            Filter::DistToArrival(dist) => {
                if let Some(d) = record.station.distance_to_arrival {
                    d <= *dist
                } else {
                    false
                }
            }
            Filter::Economy(list, include_secondary) => {
                if let Some(economy) = record.station.economy {
                    if list.contains(&economy) {
                        return true;
                    }
                }
                if let Some(second) = record.station.second_economy {
                    if *include_secondary && list.contains(&second) {
                        return true;
                    }
                }
                false
            }
            Filter::IgnorePlanetary => !record.station.st_type.is_planetary(),
            Filter::LPadOnly => record.station.st_type.has_l_pad(),
            Filter::Outdated => check_outdated(record),
            Filter::StationName(rs) => !rs.is_match(&record.station.name),
            Filter::SystemName(rs) => !rs.is_match(&record.station.system_name),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Days {
    Information(i64),
    Market(i64),
    Shipyard(i64),
    Outfitting(i64),
}

impl searcher::Filter for Days {
    fn filter<'a>(&self, record: &mut Record<'a>) -> bool {
        match self {
            Days::Information(days) => {
                record.information_days.check(|d| d >= *days);
            }
            Days::Market(days) => {
                record.market_days.check(|d| d >= *days);
            }
            Days::Shipyard(days) => {
                record.shipyard_days.check(|d| d >= *days);
            }
            Days::Outfitting(days) => {
                record.outfitting_days.check(|d| d >= *days);
            }
        }

        true
    }
}

fn check_outdated(record: &mut Record) -> bool {
    record.information_days.is_outdated()
        || record.market_days.is_outdated()
        || record.shipyard_days.is_outdated()
        || record.outfitting_days.is_outdated()
}
