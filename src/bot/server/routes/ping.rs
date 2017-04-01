use futures::{Future, IntoFuture};
use hyper::server::{Response};

use super::super::HandlerFuture;

pub fn ping() -> HandlerFuture {
    Ok(
        Response::new()
            .with_body("Beep boop.")
    ).into_future().boxed()
}
