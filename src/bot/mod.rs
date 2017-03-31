use std::net::SocketAddr;

use futures::{Future, future};
use tokio_core::reactor::Core;

use config::Config;

use self::client::Client;
use self::server::Server;
use self::work::Queue;

mod client;
mod server;
mod work;

pub struct FxaBot {
    addr: SocketAddr,
    core: Core,
}

impl FxaBot {
    pub fn new(config: Config) -> Result<FxaBot, ()> {
        // create the Core that will run the world
        let core = Core::new().unwrap();
        let handle = core.handle();
        // attach a client
        let client = Client::new(config.clone(), &handle);
        // attach a work queue
        let work = Queue::new(client, &handle);
        // attach a server
        let addr = Server::listen(config, work, &handle).unwrap();

        Ok(FxaBot {
            core: core,
            addr: addr,
        })
    }

    pub fn run(mut self) -> Result<(), ()> {
        self.run_until(future::empty::<(), ()>())
    }

    pub fn run_until<F: Future<Item=(), Error=()>>(&mut self, f: F) -> Result<(), ()> {
        self.core.run(f)
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}
