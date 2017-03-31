use futures::{Future, IntoFuture};
use hyper::server::{Request, Response};

use super::HandlerFuture;

pub use self::github::handle as github;

mod github;

pub fn ping() -> HandlerFuture {
    Ok(Response::new()).into_future().boxed()
}

#[derive(Debug)]
enum RouteError {
    Client,
    Server,
}
