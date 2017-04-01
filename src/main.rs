extern crate fxabot;
extern crate pretty_env_logger;

use fxabot::{FxaBot, Config};

fn main() {
    pretty_env_logger::init().unwrap();

    let arg = match ::std::env::args().nth(1) {
        Some(s) => s,
        None => {
            println!("Usage: {} <path>", ::std::env::args().next().unwrap());
            return;
        }
    };

    println!("boop: using config file {:?}", arg);

    match run(arg) {
        Ok(_) => {},
        Err(e) => {
            println!("beep! error: {:?}", e);
            ::std::process::exit(1);
        }
    }
}

fn run(path: String) -> Result<(), ()> {
    let config = Config::parse_file(path).expect("foo");
    let bot = FxaBot::new(config)?;
    bot.run()
}
