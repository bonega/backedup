#[macro_use]
extern crate lazy_static;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fmt;
use std::fs::read_dir;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Datelike, DateTime, TimeZone, Utc};
use regex::Regex;
use termion::{color, style};
use termion::color::Fg;

#[derive(Default)]
pub struct SlotConfig {
    pub years: usize,
    pub months: usize,
    pub days: usize,
}

impl SlotConfig {
    fn get_slot(&self, period: Period) -> usize {
        match period {
            Period::Years => { self.years }
            Period::Months => { self.months }
            Period::Days => { self.days }
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Hash, Clone, Ord)]
struct BackupEntry {
    timestamp: DateTime<Utc>,
    path: PathBuf,
}

impl Eq for BackupEntry {}


#[derive(Copy, Clone, Debug)]
pub enum Period {
    Years,
    Months,
    Days,
}

impl Period {
    fn to_string(&self) -> &'static str {
        match self {
            Period::Years => { "Years" }
            Period::Months => { "Months" }
            Period::Days => { "Days" }
        }
    }
}

lazy_static! {
    static ref RE: Regex =  Regex::new(r"(?x)(?P<year>\d{4}) \D?
(?P<month>\d{2}) \D?
(?P<day>\d{2}) \D?
(
   # Optional components.
   (?P<hour>\d{2}) \D?
   (?P<minute>\d{2}) \D?
   (?P<second>\d{2})?
)?").unwrap();
}

pub struct Plan {
    pub to_keep: Vec<(PathBuf, Vec<Period>)>,
    pub to_remove: Vec<PathBuf>,
}

impl Display for Plan {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}Plan{}", Fg(color::Blue), style::Reset)?;
        writeln!(f, "\tKeep {}/{} timestamped files\n", self.to_keep.len(), self.to_keep.len() + self.to_remove.len())?;
        writeln!(f, "\t{}Keep files matching periods", Fg(color::Green))?;
        for (i, periods) in &self.to_keep {
            write!(f, "\t\t{}{} {}", Fg(color::Green), i.to_str().unwrap(), style::Reset)?;
            let periods: Vec<_> = periods.iter().map(|x| x.to_string()).collect();
            writeln!(f, "-> ({})", periods.join(","))?;
        }
        writeln!(f, "")?;
        writeln!(f, "\t{}Remove files not matching periods", Fg(color::Red))?;
        for i in &self.to_remove {
            writeln!(f, "\t\t{}", i.to_str().unwrap())?;
        }
        Ok(())
    }
}


impl Plan {
    pub fn new<P: AsRef<Path>>(config: &SlotConfig, path: P) -> Result<Self> {
        let dir = read_dir(path)?;
        let entries: Vec<_> = dir
            .flatten()
            .filter_map(|x| BackupEntry::new(x.path()).ok())
            .collect();
        Ok(Self::from(config, &entries))
    }

    fn from(config: &SlotConfig, entries: &Vec<BackupEntry>) -> Self {
        fn insert_max<'a, K>(map: &mut HashMap<K, &'a BackupEntry>, k: K, v: &'a BackupEntry)
            where K: Eq + Hash {
            let entry = map.entry(k).or_insert(v);
            if entry.timestamp > v.timestamp {
                *entry = v;
            }
        }

        let mut year_slots = HashMap::new();
        let mut month_slots = HashMap::new();
        let mut day_slots = HashMap::new();
        for entry in entries.iter() {
            let dt = entry.timestamp;
            insert_max(&mut year_slots, dt.year(), entry);
            insert_max(&mut month_slots, (dt.year(), dt.month()), entry);
            insert_max(&mut day_slots, (dt.year(), dt.month(), dt.day()), entry);
        }
        let parsed_set: HashSet<_> = entries.into_iter().map(|x| &x.path).collect();

        let mut to_keep = BTreeMap::new();
        let mut keep_from_period = |mut slots: Vec<&&BackupEntry>, period| {
            let n = config.get_slot(period);
            slots.sort_by_key(|x| x.timestamp);
            slots
                .into_iter()
                .rev()
                .take(n)
                .for_each(|&x|
                    {
                        to_keep.entry(x.clone())
                            .or_insert(Vec::new()).push(period);
                    }
                );
        };

        keep_from_period(year_slots.values().collect(), Period::Years);
        keep_from_period(month_slots.values().collect(), Period::Months);
        keep_from_period(day_slots.values().collect(), Period::Days);

        let to_keep: Vec<_> = to_keep.into_iter().collect();
        let entries_set: HashSet<_> = entries.iter().collect();
        let to_keep_set: HashSet<_> = to_keep.iter().map(|(x, _)| x).collect();
        let mut to_remove: Vec<_> = entries_set.difference(&to_keep_set).collect();
        to_remove.sort_by_key(|x| x.timestamp);
        let to_remove: Vec<_> = to_remove.iter().map(|&x| x.path.clone()).collect();
        let to_keep: Vec<_> = to_keep.into_iter().map(|(e, p)| (e.path, p)).collect();
        assert_eq!(parsed_set.len(), &to_keep.len() + &to_remove.len());
        Self {
            to_keep,
            to_remove,
        }
    }
}

impl BackupEntry {
    fn new(path: PathBuf) -> Result<Self> {
        let timestamp = path
            .file_name()
            .context("Couldn't get filename")?
            .to_str()
            .and_then(|s| datetime_from_regex(s, &RE))
            .context("Failed to parse datetime")?;
        Ok(Self { timestamp, path })
    }
}


#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn create_test_data(mut start_dt: DateTime<Utc>, days: usize) -> Vec<BackupEntry> {
        let mut result = Vec::new();
        for _ in 0..days {
            let path = PathBuf::from(start_dt.format("%Y-%m-%d").to_string());
            let entry = BackupEntry::new(path).unwrap();
            result.push(entry);
            start_dt = start_dt - Duration::days(1);
        }
        result
    }

    #[test]
    fn test_make_plan() {
        let parsed_backups = create_test_data(Utc.ymd(2015, 1, 1)
                                                  .and_hms(0, 0, 0), 400);
        let mut config = SlotConfig {
            years: 3,
            ..Default::default()
        };

        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 3);

        config.months = 13;
        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 14);

        config.days = 30;
        let plan = Plan::from(&config, &parsed_backups);
        assert_eq!(plan.to_keep.len(), 43);
    }
}


fn datetime_from_regex(s: &str, re: &Regex) -> Option<DateTime<Utc>> {
    let m = re.captures(s)?;
    let year = m.name("year")?.as_str().parse().ok()?;
    let month = m.name("month")?.as_str().parse().ok()?;
    let day = m.name("day")?.as_str().parse().ok()?;
    let hour = m.name("hour").and_then(|s| s.as_str().parse().ok()).unwrap_or(0);
    let minute = m.name("minute").and_then(|s| s.as_str().parse().ok()).unwrap_or(0);
    let second = m.name("second").and_then(|s| s.as_str().parse().ok()).unwrap_or(0);
    Some(Utc.ymd(year, month, day).and_hms(hour, minute, second))
}