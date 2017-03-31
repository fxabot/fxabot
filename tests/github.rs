extern crate futures;
extern crate fxabot;

use std::thread;

use futures::Future;
use futures::sync::oneshot;
use fxabot::FxaBot;

use self::utils::request;
mod utils;


static TEST_CONFIG: &'static str = r#"
[server]
host = "127.0.0.1"
port = 0
"#;

#[test]
fn test_smoke() {
    let (_tx, rx) = oneshot::channel();
    let (addr_tx, addr_rx) = oneshot::channel();
    thread::spawn(move || {
        let mut bot = FxaBot::new(TEST_CONFIG.parse().unwrap()).unwrap();
        addr_tx.send(bot.addr().clone()).unwrap();
        bot.run_until(rx.then(|_| Ok(()))).unwrap();
    });

    let addr = addr_rx.wait().unwrap();

    let res = request(&addr)
        .get("/")
        .response();
    assert_eq!(res.code(), 200);
    assert_eq!(res.body(), "");

    let res = request(&addr)
        .get("/woops")
        .response();
    assert_eq!(res.code(), 404);
}
