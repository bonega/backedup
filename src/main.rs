#[macro_use]
extern crate log;
#[macro_use(slog_o)]
extern crate slog;

use anyhow::Result;
use argh::FromArgs;
use slog::{Drain, Logger};
use slog_syslog::Facility;

use backedup::{BackedUpError, Config, Plan, SlotConfig};

#[derive(FromArgs)]
///Backedup
struct ArgParser {
    #[argh(positional)]
    path: String,

    ///filename pattern to look for, quote it to prevent shell expansion.
    /// Can be provided several times
    #[argh(option)]
    include: Vec<String>,

    ///set number of backups for yearly slot
    #[argh(option, default = "0")]
    yearly: usize,

    ///set number of backups for monthly slot
    #[argh(option, default = "0")]
    monthly: usize,

    ///set number of backups for daily slot
    #[argh(option, default = "0")]
    daily: usize,

    ///set number of backups for hourly slot
    #[argh(option, default = "0")]
    hourly: usize,

    ///set number of backups for minutely slot
    #[argh(option, default = "0")]
    minutely: usize,

    ///provide alternate regex for parsing timeslots. At least year, month and day must be provided and named
    /// eg '(?P<year>\d{{2}})(?P<month>\d{{2}})(?P<day>\d{{2}})'
    #[argh(option)]
    regex: Option<String>,

    ///execute plan and remove timestamped files not matching a slot
    #[argh(switch)]
    execute: bool,

}

fn argparser_to_plan(parser: &ArgParser) -> Result<Plan, BackedUpError> {
    let slot_config = SlotConfig::new(parser.yearly,
                                      parser.monthly,
                                      parser.daily,
                                      parser.hourly,
                                      parser.minutely)?;
    let re_str = parser.regex.as_ref().map(|s| s.as_str());
    let config = Config::new(slot_config, &parser.include, re_str)?;

    Plan::new(&config, &parser.path)
}

fn init_logging() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let d1 = std::sync::Mutex::new(drain).fuse();
    let d2 = slog_syslog::unix_3164(Facility::LOG_USER).unwrap().fuse();
    let drain = slog::Duplicate(d1, d2).fuse();
    slog::Logger::root(drain, slog_o!())
}

fn main() -> Result<()> {
    let logger = init_logging();
    let _scope_guard = slog_scope::set_global_logger(logger);
    let _log_guard = slog_stdlog::init().unwrap();
    let parser = argh::from_env();
    let res = argparser_to_plan(&parser);
    if let Err(e) = &res {
        eprintln!("{}", e);
        anyhow::bail!("Couldn't construct plan");
    }

    let plan = res.unwrap();

    if parser.execute {
        if !plan.to_remove.is_empty() {
            info!("Executing plan to remove {} and keep {} files", plan.to_remove.len(), plan.to_keep.len());
        }
        plan.execute();
    } else {
        println!("{}", plan);
    }
    Ok(())
}