#[macro_use]
extern crate log;
#[macro_use(slog_o)]
extern crate slog;

use anyhow::Result;
use slog::{Drain, Logger};
use slog_syslog::Facility;

use config::ArgParser;

mod config;

fn init_logging() -> Result<Logger> {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let d1 = std::sync::Mutex::new(drain).fuse();
    let d2 = slog_syslog::unix_3164(Facility::LOG_USER)?.fuse();
    let drain = slog::Duplicate(d1, d2).fuse();
    Ok(slog::Logger::root(drain, slog_o!()))
}

fn main() -> Result<()> {
    let logger = init_logging()?;
    let _scope_guard = slog_scope::set_global_logger(logger);
    let _log_guard = slog_stdlog::init()?;
    let parser = ArgParser::new();

    let plan = parser.to_plan()?;

    if parser.execute {
        if !plan.to_remove.is_empty() {
            info!(
                "Executing plan to remove {} and keep {} files",
                plan.to_remove.len(),
                plan.to_keep.len()
            );
        }
        plan.execute()?;
    } else {
        println!("{}", plan);
    }
    Ok(())
}
