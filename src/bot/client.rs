use futures::{Future, IntoFuture};
use hyper::client::{Client as HyperClient, Request};
use hyper::{self, Method, StatusCode};
use hyper::header::{Authorization, Bearer, UserAgent};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Handle;

use config::Config;

#[derive(Clone)]
pub struct Client {
    client: HyperClient<HttpsConnector>,
    config: Config,
}


pub type Response<T> = Box<Future<Item=T, Error=Error>>;

impl Client {
    pub fn new(config: Config, handle: &Handle) -> Client {
        let client = HyperClient::configure()
                .connector(HttpsConnector::new(4, handle))
                .build(handle);
        Client {
            client: client,
            config: config,
        }
    }

    pub fn github_comment(&self, repo: String, issue: u64, body: String) -> Response<()> {
        let path = format!("/repos/{}/issues/{}/comments", repo, issue);
        let mut req = match self.request(Method::Post, &path) {
            Ok(req) => req,
            Err(e) => {
                error!("failed to parse uri: {}", e);
                return Box::new(Err(Error::Http(hyper::Error::Uri(e))).into_future());
            }
        };

        req.set_body(json!({
            "body": body
        }).to_string());

        let res = self.client.request(req)
            .map_err(From::from)
            .and_then(|res| {
                if res.status() == StatusCode::Created {
                    Ok(())
                } else {
                    error!("unexpected status code for github comment: {}", res.status());
                    Err(Error::Api)
                }
            });
        Box::new(res)
    }

    fn request(&self, method: Method, path: &str) -> Result<Request, hyper::error::UriError> {
        let uri = format!("{}{}", self.config.github_api(), path).parse()?;
        let mut req = Request::new(method, uri);

        req.headers_mut().set(UserAgent("fxabot/0".to_string()));
        if let Some(token) = self.config.github_token() {
            req.headers_mut().set(Authorization(Bearer {
                token: token.to_string(),
            }));
        }

        Ok(req)
    }
}

#[derive(Debug)]
pub enum Error {
    Http(hyper::Error),
    Api,
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Error {
        Error::Http(e)
    }
}
