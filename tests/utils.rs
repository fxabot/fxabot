extern crate futures;
extern crate hyper;
extern crate tokio_core;

use std::net::SocketAddr;

use self::futures::Stream;
use self::hyper::{Client, Uri, Method};
use self::hyper::client::{Request as HyperRequest, Response as HyperResponse};
use self::tokio_core::reactor::Core;

pub fn request(addr: &SocketAddr) -> Request {
    let core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    Request {
        addr: addr.clone(),
        core: core,
        client: client,
        request: None,
    }
}

pub struct Request {
    addr: SocketAddr,
    core: Core,
    client: Client<self::hyper::client::HttpConnector>,
    request: Option<HyperRequest>,
}

impl Request {
    pub fn get(mut self, path: &str) -> Request {
        self.request = Some(HyperRequest::new(Method::Get, self.uri(path)));
        self
    }

    pub fn response(self) -> Response {
        let req = self.request.unwrap();
        let mut core = self.core;
        let client = self.client;

        let res = core.run(client.request(req)).unwrap();
        Response {
            core: core,
            response: res,
        }
    }

    fn uri(&self, path: &str) -> Uri {
        format!("http://{}{}", self.addr, path).parse().unwrap()
    }
}

pub struct Response {
    core: Core,
    response: HyperResponse,
}

impl Response {
    pub fn code(&self) -> u16 {
        self.response.status().clone().into()
    }

    pub fn body(self) -> String {
        let mut core = self.core;
        let body = core.run(self.response.body()
            .fold(Vec::new(), |mut body, chunk| {
                body.extend_from_slice(&chunk);
                Ok::<_, self::hyper::Error>(body)
            })
        ).unwrap();
        String::from_utf8(body).unwrap()
    }
}
