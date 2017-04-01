pub use self::github::handle as github;
pub use self::ping::ping;

mod github;
mod ping;

#[derive(Debug)]
enum RouteError {
    Client,
    Server,
}
