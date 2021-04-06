use anyhow::Result;
use argh::FromArgs;

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

    ///execute plan and removes timestamped files not matching a slot
    #[argh(switch)]
    execute: bool,

}

fn argparser_to_plan(parser: &ArgParser) -> Result<Plan, BackedUpError> {
    let slot_config = SlotConfig::new(parser.yearly,
                                      parser.monthly,
                                      parser.daily,
                                      parser.hourly,
                                      parser.minutely)?;
    let config = Config::new(slot_config, &parser.include);

    Plan::new(&config, &parser.path)
}


fn main() -> Result<()> {
    let parser = argh::from_env();
    let res = argparser_to_plan(&parser);
    if let Err(e) = &res {
        eprintln!("{}", e);
        anyhow::bail!("Couldn't construct plan");
    }

    let plan = res.unwrap();

    if parser.execute {
        let _ = plan.execute();
    } else {
        println!("{}", plan);
    }
    Ok(())
}