use std::fmt;

use futures::{Future, IntoFuture, Stream};
use hmacsha1::hmac_sha1;
use hyper::{self, StatusCode};
use hyper::header::{Header, Raw};
use hyper::server::{Request, Response};
use serde_json;

use config::Config;
use bot::work::{Job, Queue};
use super::super::HandlerFuture;
use super::RouteError;

pub fn handle(config: Config, work: Queue, req: Request) -> HandlerFuture {
    GithubHandler {
        config: config,
        work: work,
    }.handle_request(req)
}

struct GithubHandler {
    config: Config,
    work: Queue,
}

impl GithubHandler {
    fn handle_request(self, req: Request) -> HandlerFuture {
        let event = match req.headers().get() {
            Some(event) => *event,
            None => return Ok(Response::new().with_status(hyper::BadRequest)).into_future().boxed(),
        };
        let sig = req.headers().get::<XHubSignature>().map(|h| h.clone());

        let body = Vec::new();
        req.body().fold(body, move |mut body, chunk| {
            body.extend_from_slice(&chunk);
            Ok::<_, hyper::Error>(body)
        }).map_err(|err| {
            error!("request body error: {}", err);
            RouteError::Client
        }).and_then(move |body| {
            if self.config.github_webhook_secret().is_some() {
                if let Some(sig) = sig {
                    self.verify_signature(&body, &sig)?;
                } else {
                    debug!("no X-Hub-Signature, rejecting");
                    return Err(RouteError::Client);
                }
            } else {
                warn!("no webhook secret configured, unknown event origin");
            }

            match event {
                XGithubEvent::IssueComment => self.handle_issue_comment(body)
            }
        }).or_else(|err| {
            let status = match err {
                RouteError::Client => StatusCode::BadRequest,
                RouteError::Server => StatusCode::InternalServerError,
            };
            Ok(Response::new().with_status(status))
        }).boxed()
    }

    fn verify_signature(&self, body: &[u8], sig: &XHubSignature) -> Result<(), RouteError> {
        if let Some(secret) = self.config.github_webhook_secret() {
            trace!("verifying signature: {:?}", sig.0);
            let digest = hmac_sha1(secret.as_bytes(), body);
            let digest = format!("{:x}", Hex(&digest));
            let prefix = b"sha1=";
            let len = digest.len() + prefix.len();
            let sig_ascii = sig.0.as_bytes();
            if sig_ascii.len() != len {
                debug!("signature not long enough");
                Err(RouteError::Client)
            } else if !(b"sha1=" == &sig_ascii[..prefix.len()] && digest.as_bytes() == &sig_ascii[prefix.len()..]) {
                error!("signature does not match, ours = {:?}, theirs = {:?}",
                       digest, sig);
                Err(RouteError::Client)
            } else {
                trace!("valid signature");
                Ok(())
            }
        } else {
            warn!("I don't have a webhook secret, I can't verify this event!");
            Ok(())
        }
    }

    fn handle_issue_comment(self, bytes: Vec<u8>) -> Result<Response, RouteError> {
        let event: CommentEvent = match serde_json::from_slice(&bytes) {
            Ok(ev) => ev,
            Err(e) => {
                error!("error decoding json: {}", e);
                return Err(RouteError::Client)
            },
        };

        trace!("event: {:?}", event);
        let cmd = Cmd::parse(&self.config, &event);
        match cmd {
            Cmd::Ping => {
                let mut job = Job::new();
                job.comment(
                    event.repository.full_name,
                    event.issue.number,
                    format!("@{} pong :ping_pong:", event.sender.login)
                );
                self.work.schedule(job).map_err(|_| RouteError::Server)?;
            },
            Cmd::Deploy => {
                let mut job = Job::new();
                job.comment(
                    event.repository.full_name,
                    event.issue.number,
                    format!("@{} I'd love to... but I don't have that chip installed yet. :sob:", event.sender.login)
                );
                self.work.schedule(job).map_err(|_| RouteError::Server)?;

            },
            Cmd::DidNotUnderstand => {
                // authorized user, but bad command
                let mut job = Job::new();
                job.comment(
                    event.repository.full_name,
                    event.issue.number,
                    format!("@{} I'm sorry, I didn't understand you. Bzzt. :zap:", event.sender.login)
                );
                self.work.schedule(job).map_err(|_| RouteError::Server)?;
            }
            Cmd::Ignore => {
                debug!("ignoring comment");
            },
        }
        Ok(Response::new())
    }
}

// The 'X-Github-Event' header
// Variants are the kinds of events we care about
#[derive(Debug, Clone, Copy)]
enum XGithubEvent {
    IssueComment,
}

impl Header for XGithubEvent {
    fn header_name() -> &'static str {
        "X-Github-Event"
    }

    fn parse_header(raw: &Raw) -> hyper::Result<XGithubEvent> {
        match raw.one() {
            Some(b"issue_comment") => Ok(XGithubEvent::IssueComment),
            _ => Err(hyper::Error::Header),
        }
    }

    fn fmt_header(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("issue_comment")
    }
}

#[derive(Debug, Clone)]
struct XHubSignature(String);

impl Header for XHubSignature {
    fn header_name() -> &'static str {
        "X-Hub-Signature"
    }

    fn parse_header(raw: &Raw) -> hyper::Result<XHubSignature> {
        match raw.one() {
            Some(bytes) => Ok(XHubSignature(::std::str::from_utf8(bytes)?.to_string())),
            _ => Err(hyper::Error::Header),
        }
    }

    fn fmt_header(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Deserialize)]
struct CommentEvent {
    action: CommentAction,
    comment: Comment,
    issue: Issue,
    repository: Repository,
    sender: User,
}

#[derive(Debug, PartialEq, Deserialize)]
enum CommentAction {
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "edited")]
    Edited,
    #[serde(rename = "deleted")]
    Deleted,
}

#[derive(Debug, Deserialize)]
struct Issue {
    number: u64,
    title: String,
}

#[derive(Debug, Deserialize)]
struct Comment {
    body: String,
    user: User,
}

#[derive(Debug, Deserialize)]
struct Repository {
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct User {
    login: String,
}

#[derive(Debug)]
enum Cmd {
    Ping,
    Deploy,
    DidNotUnderstand,
    Ignore,
}

impl Cmd {
    fn parse(config: &Config, event: &CommentEvent) -> Cmd {
        if event.action != CommentAction::Created {
            return Cmd::Ignore;
        }
        let my_name = config.github_name();
        if my_name.is_empty() {
            return Cmd::Ignore;
        }
        let authorized = config.github_authorized();
        if let Some(line) = Cmd::find_mention(&event.comment.body, my_name) {
            debug!("someone mentioned me: {:?}", line);
            if authorized.contains(&event.sender.login) {
                Cmd::parse_line(line)
            } else {
                debug!("not someone I trust: {:?}", event.sender.login);
                Cmd::Ignore
            }
        } else {
            Cmd::Ignore
        }

    }

    fn find_mention<'a>(body: &'a str, my_name: &str) -> Option<&'a str> {
        let min_len = 1 + my_name.len() + 1; // '@my_name '
        body.split('\n')
            .skip_while(|line| {
                !(
                    line.len() > min_len
                    && line.starts_with('@')
                    && line[1..].starts_with(my_name)
                    && line.as_bytes()[min_len - 1] == b' '
                )
            })
            .next()
    }

    fn parse_line(line: &str) -> Cmd {
        let mut words = line.split(' ');
        let _name = words.next();
        //TODO: assert name is @my_name

        match words.next() {
            Some("ping") => Cmd::Ping,
            Some("deploy") => Cmd::Deploy,
            _ => Cmd::DidNotUnderstand,
        }
    }
}

struct Hex<'a>(&'a [u8]);

impl<'a> fmt::LowerHex for Hex<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            fmt::LowerHex::fmt(byte, f)?
        }
        Ok(())
    }
}
