use regex::RegexSet;

use crate::searcher::{self, Record};

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
    Outdated,
    StationName(RegexSet),
    SystemName(RegexSet),
}

impl searcher::Filter for Filter {
    fn filter<'a>(&self, record: &mut Record<'a>) -> bool {
        match self {
            Filter::Days(days) => days.filter(record),
            Filter::Dist(dist) => record.distance <= *dist,
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
