#[macro_use]
extern crate lazy_static;

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::{read_dir, remove_file};
use std::hash::Hash;
use std::path::{Path, PathBuf};

use log::{error, info};
use regex::Regex;
use termion::{color, style};
use termion::color::Fg;
use thiserror::Error;
use wildmatch::WildMatch;

#[derive(Error, Debug, PartialEq)]
pub enum BackedUpError {
    #[error("No such directory \"{path}\"")]
    ReadDirError { path: PathBuf },
    #[error("At least one slot must be configured")]
    NoSlot,
    #[error("Invalid regex")]
    InvalidRegex,
    #[error("Regex missing capture group for \"{name}\". -- example: (?P<{name}>\\d{{2}})")]
    MissingCaptureGroup { name: String },
}

#[derive(Copy, Clone)]
pub struct SlotConfig {
    yearly: usize,
    monthly: usize,
    daily: usize,
    hourly: usize,
    minutely: usize,
}

impl SlotConfig {
    pub fn new(
        years: usize,
        months: usize,
        days: usize,
        hours: usize,
        minutes: usize,
    ) -> Result<Self, BackedUpError> {
        if years + months + days + hours + minutes == 0 {
            return Err(BackedUpError::NoSlot);
        }
        Ok(Self {
            yearly: years,
            monthly: months,
            daily: days,
            hourly: hours,
            minutely: minutes,
        })
    }
}

pub struct Config {
    slots: SlotConfig,
    pattern: Vec<WildMatch>,
    re: Regex,
}

impl Config {
    pub fn new(
        slot_config: SlotConfig,
        pattern: &[&str],
        re_str: Option<&str>,
    ) -> Result<Self, BackedUpError> {
        let pattern = pattern.iter().map(|s| WildMatch::new(s)).collect();
        let re = match re_str {
            None => (*RE).clone(),
            Some(s) => Regex::new(s).map_err(|_| BackedUpError::InvalidRegex)?,
        };
        let capture_names: Vec<_> = re.capture_names().flatten().collect();
        for i in ["year", "month", "day"].iter() {
            if !capture_names.contains(i) {
                return Err(BackedUpError::MissingCaptureGroup {
                    name: i.to_string(),
                });
            }
        }
        Ok(Self {
            slots: slot_config,
            pattern,
            re,
        })
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash)]
struct BackupEntry<'a> {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    path: &'a Path,
}

impl<'a> BackupEntry<'a> {
    fn new(path: &'a Path, config: &Config) -> Option<Self> {
        let pattern = &config.pattern;
        let filename = path.file_name()?.to_str()?;
        if !pattern.is_empty() && !pattern.iter().any(|w| w.matches(filename)) {
            return None;
        }
        let m = &config.re.captures(filename)?;
        let year = m.name("year")?.as_str().parse().ok()?;
        let month = m.name("month")?.as_str().parse().ok()?;
        let day = m.name("day")?.as_str().parse().ok()?;
        let hour = m
            .name("hour")
            .and_then(|s| s.as_str().parse().ok())
            .unwrap_or(0);
        let minute = m
            .name("minute")
            .and_then(|s| s.as_str().parse().ok())
            .unwrap_or(0);
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            path,
        })
    }

    fn get_ordering_tuple(&self) -> (u16, u8, u8, u8, u8) {
        (self.year, self.month, self.day, self.hour, self.minute)
    }
}

impl<'a> Ord for BackupEntry<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_ordering_tuple().cmp(&other.get_ordering_tuple())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Period {
    Years,
    Months,
    Days,
    Hours,
    Minutes,
}

impl Period {
    fn to_string(&self) -> &'static str {
        match self {
            Period::Years => "Years",
            Period::Months => "Months",
            Period::Days => "Days",
            Period::Hours => "Hours",
            Period::Minutes => "Minutes",
        }
    }
}

lazy_static! {
    static ref RE: Regex = Regex::new(
        r"(?x)(?P<year>\d{4}) \D?
(?P<month>\d{2}) \D?
(?P<day>\d{2}) \D?
(
   # Optional components.
   (?P<hour>\d{2}) \D?
   (?P<minute>\d{2}) \D?
   (?P<second>\d{2})?
)?"
    )
    .unwrap();
}

/// Plan for keeping/removing PathBuf with configured slots.
/// PathBufs that are invalid strings aren't considered for either removal or keep
pub struct Plan {
    pub to_keep: Vec<PathBuf>,
    pub to_remove: Vec<PathBuf>,
    period_map: HashMap<PathBuf, Vec<Period>>,
}

impl Display for Plan {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Plan to:\n")?;
        if self.to_keep.is_empty() && self.to_remove.is_empty() {
            writeln!(f, "\tDo nothing: no valid timestamps")?;
            return Ok(());
        }
        writeln!(
            f,
            "\t{}Keep {} file(s) matching {}period(s)",
            Fg(color::Green),
            &self.to_keep.len(),
            style::Reset
        )?;
        for i in &self.to_keep {
            write!(
                f,
                "\t\t{}{} {}",
                Fg(color::Green),
                i.to_str().unwrap(),
                style::Reset
            )?;
            let periods = self.period_map.get(i).unwrap();
            let periods: Vec<_> = periods.iter().map(|x| x.to_string()).collect();
            writeln!(f, "-> ({})", periods.join(","))?;
        }
        writeln!(f, "")?;
        writeln!(
            f,
            "\t{}Remove {} file(s) not matching periods",
            Fg(color::Red),
            &self.to_remove.len()
        )?;
        for i in &self.to_remove {
            writeln!(f, "\t\t{}", i.to_str().unwrap())?;
        }
        Ok(())
    }
}

impl Plan {
    pub fn new<P: AsRef<Path>>(config: &Config, path: P) -> Result<Self, BackedUpError> {
        let dir = read_dir(&path).map_err(|_| BackedUpError::ReadDirError {
            path: path.as_ref().to_path_buf(),
        })?;
        let entries: Vec<_> = dir.flatten().map(|x| x.path()).collect();
        Ok(Self::from(&config, &entries))
    }

    fn from(config: &Config, entries: &[PathBuf]) -> Self {
        let entries: BTreeSet<_> = entries
            .into_iter()
            .filter_map(|x| BackupEntry::new(x, config))
            .collect();
        let mut year_slots = BTreeMap::new();
        let mut month_slots = BTreeMap::new();
        let mut day_slots = BTreeMap::new();
        let mut hour_slots = BTreeMap::new();
        let mut minute_slots = BTreeMap::new();
        for entry in entries.iter().rev() {
            year_slots.insert(entry.year, entry);
            month_slots.insert((entry.year, entry.month), entry);
            day_slots.insert((entry.year, entry.month, entry.day), entry);
            hour_slots.insert((entry.year, entry.month, entry.day, entry.hour), entry);
            minute_slots.insert(
                (entry.year, entry.month, entry.day, entry.hour, entry.minute),
                entry,
            );
        }

        let mut to_keep = BTreeSet::new();
        let mut period_map: HashMap<PathBuf, Vec<Period>> = HashMap::new();
        let SlotConfig {
            yearly,
            monthly,
            daily,
            hourly,
            minutely,
        } = config.slots;
        for (_, entry) in year_slots.into_iter().rev().take(yearly) {
            to_keep.insert(entry.clone());
            period_map
                .entry(entry.path.to_path_buf())
                .or_default()
                .push(Period::Years);
        }
        for (_, entry) in month_slots.into_iter().rev().take(monthly) {
            to_keep.insert(entry.clone());
            period_map
                .entry(entry.path.to_path_buf())
                .or_default()
                .push(Period::Months);
        }
        for (_, entry) in day_slots.into_iter().rev().take(daily) {
            to_keep.insert(entry.clone());
            period_map
                .entry(entry.path.to_path_buf())
                .or_default()
                .push(Period::Days);
        }
        for (_, entry) in hour_slots.into_iter().rev().take(hourly) {
            to_keep.insert(entry.clone());
            period_map
                .entry(entry.path.to_path_buf())
                .or_default()
                .push(Period::Hours);
        }
        for (_, entry) in minute_slots.into_iter().rev().take(minutely) {
            to_keep.insert(entry.clone());
            period_map
                .entry(entry.path.to_path_buf())
                .or_default()
                .push(Period::Minutes);
        }

        let to_remove: Vec<_> = entries
            .difference(&to_keep)
            .map(|x| x.path.to_path_buf())
            .collect();
        let to_keep: Vec<_> = to_keep.into_iter().map(|x| x.path.to_path_buf()).collect();
        assert_eq!(entries.len(), &to_keep.len() + &to_remove.len());
        Self {
            to_keep,
            to_remove,
            period_map,
        }
    }

    /// Execute plan and remove timestamped files not matching any slots
    pub fn execute(&self) {
        if self.to_remove.is_empty() {
            info!("No file to remove")
        }
        for p in self.to_remove.iter() {
            let filename = p.to_str().unwrap();
            match remove_file(p) {
                Ok(_) => {
                    info!("removed file {}", filename)
                }
                Err(_) => {
                    error!("failed to remove file {}", filename)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, TimeZone, Utc};

    use super::*;

    fn create_test_data(
        fmt: &str,
        mut start_dt: DateTime<Utc>,
        days: usize,
        extension: &str,
    ) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let fmt = format!("{}{}", fmt, extension);
        for _ in 0..days {
            let path = PathBuf::from(start_dt.format(fmt.as_str()).to_string());
            result.push(path);
            start_dt = start_dt - Duration::days(1);
        }
        result
    }

    #[test]
    fn test_make_plan() {
        let fmt = "%Y-%m-%d";
        let mut parsed_backups =
            create_test_data(fmt, Utc.ymd(2015, 1, 1).and_hms(0, 0, 0), 400, "");

        // no effect for number of matches until changing pattern
        parsed_backups.append(&mut create_test_data(
            fmt,
            Utc.ymd(2015, 1, 1).and_hms(0, 0, 0),
            30,
            ".log",
        ));
        let slot_config = SlotConfig::new(3, 0, 0, 0, 0).unwrap();
        let mut config = Config::new(slot_config, &vec![], None).unwrap();

        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 3);

        config.slots.monthly = 13;
        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 14);

        config.slots.daily = 30;
        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 43);

        config.pattern = vec![WildMatch::new("*.log")];
        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 30);
    }

    #[test]
    fn test_custom_regex() {
        let fmt = "%y%m%d";
        let parsed_backups = create_test_data(fmt, Utc.ymd(2015, 1, 1).and_hms(0, 0, 0), 400, "");
        let slot_config = SlotConfig::new(3, 13, 30, 0, 0).unwrap();
        let re_str = r"(?P<year>\d{2})(?P<month>\d{2})(?P<day>\d{2})";
        let config = Config::new(slot_config, &vec![], Some(re_str)).unwrap();
        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 43);
    }

    #[test]
    fn test_no_slot() {
        let slot_config = SlotConfig::new(0, 0, 0, 0, 0);
        assert_eq!(BackedUpError::NoSlot, slot_config.err().unwrap());
    }

    #[test]
    fn test_missing_named_group() {
        let slot_config = SlotConfig::new(1, 0, 0, 0, 0).unwrap();
        let re_str = r"(?P<month>\d{2})(?P<day>\d{2})";

        let config = Config::new(slot_config, &vec![], Some(re_str));
        assert_eq!(
            BackedUpError::MissingCaptureGroup {
                name: "year".to_string()
            },
            config.err().unwrap()
        );

        let re_str = r"(?P<year>\d{2})(?P<day>\d{2})";
        let config = Config::new(slot_config, &vec![], Some(re_str));
        assert_eq!(
            BackedUpError::MissingCaptureGroup {
                name: "month".to_string()
            },
            config.err().unwrap()
        );

        let re_str = r"(?P<year>\d{2})(?P<month>\d{2})";
        let config = Config::new(slot_config, &vec![], Some(re_str));
        assert_eq!(
            BackedUpError::MissingCaptureGroup {
                name: "day".to_string()
            },
            config.err().unwrap()
        );
    }
}
