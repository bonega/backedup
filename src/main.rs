use argh::FromArgs;

use backedup::{BackedUpError, SlotConfig, Plan};

#[derive(FromArgs)]
///Backedup
struct ArgParser {
    #[argh(positional)]
    path: String,

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

}

fn argparser_to_plan(parser: ArgParser) -> Result<Plan, BackedUpError> {
    let config = SlotConfig::new(parser.yearly,
                                 parser.monthly,
                                 parser.daily,
                                 parser.hourly,
                                 parser.minutely)?;

    Plan::new(&config, parser.path)
}


fn main() {
    let res = argparser_to_plan(::argh::from_env());
    match res {
        Ok(plan) => { println!("{}", plan); }
        Err(e) => { eprintln!("{}", e) }
    }
}

    let res = Plan::new(&config, "./").unwrap();
    println!("{}", res);
}