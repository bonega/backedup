use anyhow::Result;
use argh::FromArgs;
use env_logger::Env;
use log::info;

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
    let config = Config::new(slot_config, &parser.include);

    Plan::new(&config, &parser.path)
}


fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
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