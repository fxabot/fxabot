use std::fmt;

use futures::{Future, IntoFuture, Stream};
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

        let body = Vec::new();
        req.body().fold(body, move |mut body, chunk| {
            body.extend_from_slice(&chunk);
            Ok::<_, hyper::Error>(body)
        }).map_err(|err| {
            error!("request body error: {}", err);
            RouteError::Client
        }).and_then(move |body| {
            // TODO: verify body against HMAC

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
                    format!("@{} pong", event.sender.login)
                );
                self.work.schedule(job).map_err(|_| RouteError::Server)?;
            },
            Cmd::DidNotUnderstand => {
                // authorized user, but bad command
                let mut job = Job::new();
                job.comment(
                    event.repository.full_name,
                    event.issue.number,
                    format!("@{} I'm sorry, I didn't understand you. Bzzt.", event.sender.login)
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

#[derive(Debug, Deserialize)]
struct CommentEvent {
    action: CommentAction,
    comment: Comment,
    issue: Issue,
    repository: Repository,
    sender: User,
}

#[derive(Debug, Deserialize)]
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
    DidNotUnderstand,
    Ignore,
}

impl Cmd {
    fn parse(config: &Config, event: &CommentEvent) -> Cmd {
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
            _ => Cmd::DidNotUnderstand,
        }
    }
}
