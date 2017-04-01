extern crate futures;
extern crate hmacsha1;
extern crate hyper;
extern crate hyper_tls;
#[macro_use] extern crate log;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate tokio_core;
extern crate toml;

mod bot;
mod config;

pub use self::config::Config;
pub use self::bot::FxaBot;
