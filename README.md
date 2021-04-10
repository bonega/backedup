# README

## BackedUp

A command line util for backup rotation.

Uses filenames to extract timestamps and then sort files into timeslots for retention or deletion.

## Installation

```bash
cargo install backedup
```

A local clone can be built from source and installed with:

```bash
cargo install .
```

## Usage

By default, the plan will be displayed. Removal only happens if the `--execute` flag is given.

Filenames from a given directory are parsed into a time representation.  
At least `year`, `month` and `day` have to present to be considered for removal. A configuration of how many files to
keep in each slot have to be provided for at least one slot.

The following command will display a plan to keep:

* one file per year for 20 years
* one file per month for 12 months
* one file per day for 30 days

```bash
backedup -y 20 -m 12 -d 30 path/to/directory
```

Files are grouped into separate slots only by time, rest of filename is never considered.  
Use `--pattern` or `-p` flag if only a specific filename pattern should be accepted.  
For example: `--pattern '*.log'`.

An alternative regex for parsing time from filenames can be provided by `--regex`. The default is

```regexp
(?P<year>\d{4}) \D?
(?P<month>\d{2}) \D?
(?P<day>\d{2}) \D?
(
# Optional components.
(?P<hour>\d{2}) \D?
(?P<minute>\d{2}) \D?
(?P<second>\d{2})?
)?
```

See `--help` for more details or look at
the [example config](https://github.com/bonega/backedup/blob/master/docs/example_config.toml)

## Logging

By default, file deletion is logged to Syslog

### misc

Regex and general inspiration is taken from [python-rotate-backups](https://github.com/xolox/python-rotate-backups)