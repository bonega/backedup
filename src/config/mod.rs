use anyhow::Error;
use clap::Parser;

use backedup::{Config, Plan, SlotConfig};

use crate::config;

mod file;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
///Backedup
pub struct ArgParser {
    path: String,

    #[clap(short, long)]
    config: Option<String>,

    ///wildcard filename pattern to look for, quote it to prevent shell expansion.
    /// Can be provided several times
    #[clap(short, long)]
    pattern: Vec<String>,

    ///set number of backups for yearly slot
    #[clap(default_value_t = 0, short, long)]
    yearly: usize,

    ///set number of backups for monthly slot
    #[clap(default_value_t = 0, short, long)]
    monthly: usize,

    ///set number of backups for daily slot
    #[clap(default_value_t = 0, short, long)]
    daily: usize,

    ///set number of backups for hourly slot
    #[clap(default_value_t = 0, short, long)]
    hourly: usize,

    ///set number of backups for minutely slot
    #[clap(default_value_t = 0, short('M'), long)]
    minutely: usize,

    ///provide alternate regex for parsing timeslots. At least year, month and day must be provided and named
    /// eg '(?P<year>\d{{2}})(?P<month>\d{{2}})(?P<day>\d{{2}})'
    #[clap(short, long)]
    regex: Option<String>,

    ///execute plan and remove timestamped files not matching a slot
    #[clap(short, long)]
    pub(crate) execute: bool,
}

impl ArgParser {
    pub fn new() -> Self {
        ArgParser::parse()
    }

    pub fn to_plan(&self) -> anyhow::Result<Plan> {
        let config = match &self.config {
            Some(s) => config::file::from(s)?,
            None => {
                let slot_config = SlotConfig::new(
                    self.yearly,
                    self.monthly,
                    self.daily,
                    self.hourly,
                    self.minutely,
                )?;
                Config::new(slot_config, &self.pattern, self.regex.as_deref())?
            }
        };

        Plan::new(&config, &self.path).map_err(Error::new)
    }
}
