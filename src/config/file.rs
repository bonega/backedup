use anyhow::{Context, Error};
use serde_derive::Deserialize;

use backedup::{Config, SlotConfig};

#[derive(Deserialize, Debug)]
struct SlotConfiguration {
    yearly: usize,
    monthly: usize,
    daily: usize,
    #[serde(default)]
    hourly: usize,
    #[serde(default)]
    minutely: usize,
}

#[derive(Deserialize, Debug)]
struct ConfigFile {
    slots: SlotConfiguration,
    #[serde(default)]
    pattern: Vec<String>,
    regex: Option<String>,
}

pub fn from(path: &str) -> anyhow::Result<Config> {
    let data = std::fs::read_to_string(path).context("Can't read config from file")?;
    let config: ConfigFile = toml::from_str(&data).context("Problem parsing config")?;
    let slots = &config.slots;
    let slot_config = SlotConfig::new(
        slots.yearly,
        slots.monthly,
        slots.daily,
        slots.hourly,
        slots.minutely,
    )?;
    Config::new(slot_config, &config.pattern, config.regex.as_deref()).map_err(Error::new)
}

#[cfg(test)]
mod tests {
    use crate::config::file::ConfigFile;

    #[test]
    fn test_read_config() {
        let s = std::fs::read_to_string("src/config/test-config.toml").unwrap();
        let config: ConfigFile = toml::from_str(&s).unwrap();
        assert_eq!(config.pattern, vec!["*.log"]);
        assert_eq!(
            config.regex.unwrap(),
            r"(?P<year>\d{2})(?P<month>\d{2})(?P<day>\d{2})"
        );
        assert_eq!(config.slots.yearly, 20);
        assert_eq!(config.slots.monthly, 12);
        assert_eq!(config.slots.daily, 30);
        assert_eq!(config.slots.hourly, 24);
        assert_eq!(config.slots.minutely, 60);
    }
}
