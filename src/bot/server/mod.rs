use std::io;
use std::net::SocketAddr;

use futures::future::{self, Future};
use futures::Stream;
use hyper;
use hyper::Method::{Get, Post};
use hyper::server::{Http, Request, Response, Service};

use tokio_core::net::TcpListener;
use tokio_core::reactor::Handle;

use config::Config;
use bot::work::Queue;

mod routes;

pub struct Server {
    _inner: (),
}

impl Server {
    pub fn listen(config: Config, work: Queue, handle: &Handle) -> io::Result<SocketAddr> {
        let listener = TcpListener::bind(&config.server_addr(), handle)?;
        let addr = listener.local_addr()?;
        let http = Http::new();
        let h = handle.clone();
        handle.spawn(listener.incoming().for_each(move |(socket, addr)| {
            http.bind_connection(&h, socket, addr, Handler {
                config: config.clone(),
                work: work.clone(),
            });
            Ok(())
        }).map_err(|e| {
            error!("listener error: {}", e);
            ()
        }));
        info!("listening to http://{}", addr);
        Ok(addr)
    }
}

struct Handler {
    config: Config,
    work: Queue,
}

type HandlerFuture = Box<Future<Item=Response, Error=hyper::Error>>;

impl Service for Handler {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = HandlerFuture;

    fn call(&self, req: Self::Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/") => routes::ping(),
            (&Post, "/github") if !self.config.github_name().is_empty() => {
                routes::github(self.config.clone(), self.work.clone(), req)
            },
            _ => future::ok(Response::new().with_status(hyper::NotFound)).boxed()
        }
    }
}
