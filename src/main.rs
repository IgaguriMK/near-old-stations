use tiny_fail::{ErrorMessageExt, Fail};

use near_old_stations::config::Config;
use near_old_stations::printer::TextPrinter;
use near_old_stations::stations::load_stations;

fn main() {
    if let Err(e) = w_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn w_main() -> Result<(), Fail> {
    let cfg = Config::load().err_msg("failed load config")?;

    let get_loc_func = cfg.get_loc_func();
    let stations = load_stations().err_msg("failed load stations dump file")?;
    let filter = cfg.filter()?;
    let printer = TextPrinter::new();
    let mode = cfg.mode();

    mode.run(stations, get_loc_func, filter, printer, cfg.max_entries())?;

    Ok(())
}
