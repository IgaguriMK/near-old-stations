pub mod text;

pub use text::TextPrinter;

use chrono::{DateTime, Utc};
use tiny_fail::Fail;

use crate::searcher::Record;

pub trait Printer {
    fn print(
        &mut self,
        records: &[Record],
        limit: usize,
        last_mod: DateTime<Utc>,
    ) -> Result<(), Fail>;

    fn clear(&mut self) -> Result<(), Fail>;
}

fn si_fmt(x: Option<f64>) -> String {
    match x {
        None => "unknown".to_owned(),
        Some(x) if x < 100.0 => format!("{:.2} ", x),
        Some(x) if x < 1_000.0 => format!("{:.1} ", x),
        Some(x) if x < 10_000.0 => format!("{:.2}k", x / 1000.0),
        Some(x) if x < 100_000.0 => format!("{:.1}k", x / 1000.0),
        Some(x) => format!("{:.0}k", x / 1000.0),
    }
}
