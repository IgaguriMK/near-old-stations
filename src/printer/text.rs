use chrono::{DateTime, Local, Utc};
use tiny_fail::Fail;

use super::{si_fmt, Printer};
use crate::searcher::Record;

#[derive(Debug, Default, Clone)]
pub struct TextPrinter {}

impl TextPrinter {
    pub fn new() -> TextPrinter {
        TextPrinter {}
    }
}

impl Printer for TextPrinter {
    fn print(
        &mut self,
        records: &[Record],
        limit: usize,
        last_mod: DateTime<Utc>,
    ) -> Result<(), Fail> {
        let s = last_mod.with_timezone(&Local).format("%F %T %Z");
        println!("Total {} stations. Last update is {}.", records.len(), s);

        for (i, r) in records.iter().enumerate() {
            if i == limit {
                break;
            }

            let mut outdated = String::with_capacity(4);
            outdated.push(if r.information_days.is_outdated() {
                'I'
            } else {
                ' '
            });
            outdated.push(if r.market_days.is_outdated() {
                'M'
            } else {
                ' '
            });
            outdated.push(if r.shipyard_days.is_outdated() {
                'S'
            } else {
                ' '
            });
            outdated.push(if r.outfitting_days.is_outdated() {
                'O'
            } else {
                ' '
            });

            println!(
                "{:>3}{:<2}{:>6.2} Ly + {:>8} Ls  {}d [{}]  {:<25} {:<12} ({})",
                i + 1,
                if r.visited { "*" } else { " " },
                r.distance,
                si_fmt(r.station.distance_to_arrival),
                r.outdated().unwrap(),
                outdated,
                r.station.name,
                r.station.system_name,
                r.station.st_type,
            );
        }

        Ok(())
    }

    fn clear(&mut self) -> Result<(), Fail> {
        println!("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n");
        Ok(())
    }
}
