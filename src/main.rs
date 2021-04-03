use backedup::{Plan, SlotConfig};

fn main() {
    let config = SlotConfig {
        years: 1,
        ..Default::default()
    };

    let res = Plan::new(&config, "./").unwrap();
    println!("{}", res);
}