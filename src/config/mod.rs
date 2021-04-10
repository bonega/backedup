use anyhow::Error;
use argh::FromArgs;

use backedup::{Config, Plan, SlotConfig};

use crate::config;

mod file;

#[derive(FromArgs)]
///Backedup
pub struct ArgParser {
    #[argh(positional)]
    path: String,

    ///config file
    #[argh(option, short = 'c')]
    config: Option<String>,

    ///wildcard filename pattern to look for, quote it to prevent shell expansion.
    /// Can be provided several times
    #[argh(option, short = 'p')]
    pattern: Vec<String>,

    ///set number of backups for yearly slot
    #[argh(option, default = "0", short = 'y')]
    yearly: usize,

    ///set number of backups for monthly slot
    #[argh(option, default = "0", short = 'm')]
    monthly: usize,

    ///set number of backups for daily slot
    #[argh(option, default = "0", short = 'd')]
    daily: usize,

    ///set number of backups for hourly slot
    #[argh(option, default = "0", short = 'h')]
    hourly: usize,

    ///set number of backups for minutely slot
    #[argh(option, default = "0", short = 'M')]
    minutely: usize,

    ///provide alternate regex for parsing timeslots. At least year, month and day must be provided and named
    /// eg '(?P<year>\d{{2}})(?P<month>\d{{2}})(?P<day>\d{{2}})'
    #[argh(option)]
    regex: Option<String>,

    ///execute plan and remove timestamped files not matching a slot
    #[argh(switch)]
    pub(crate) execute: bool,
}

impl ArgParser {
    pub fn new() -> Self {
        argh::from_env()
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

        Plan::new(&config, &self.path).map_err(|e| Error::new(e))
    }
}
