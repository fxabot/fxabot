use futures::{Future, Stream, stream};
use futures::sync::mpsc;

use tokio_core::reactor::Handle;

use bot::client::Client;

#[derive(Debug, Clone)]
pub struct Queue {
    tx: mpsc::UnboundedSender<Job>,
}

impl Queue {
    pub fn new(client: Client, handle: &Handle) -> Queue {
        let (tx, rx) = mpsc::unbounded();
        let jobs = Jobs {
            client: client,
            handle: handle.clone(),
        };
        handle.spawn(rx.for_each(move |job| {
            jobs.on_recv(job);
            Ok(())
        }));
        Queue {
            tx: tx
        }
    }


    pub fn schedule(&self, job: Job) -> Result<(), Job> {
        self.tx.send(job)
            .map_err(|e| e.into_inner())
    }
}

struct Jobs {
    client: Client,
    handle: Handle,
}

impl Jobs {
    fn on_recv(&self, job: Job) {
        trace!("queuing new job: {:?}", job);

        let client = self.client.clone();
        let stream = stream::iter(job.tasks.into_iter().map(|t| Ok(t))).for_each(move |task| {
            match task {
                Task::GithubComment { repo, issue, body } => {
                    client.github_comment(repo, issue, body)
                        .map(|_| ())
                        .map_err(|e| {
                            error!("task failed: {:?}", e);
                            ()
                        })
                }
            }
        });

        self.handle.spawn(stream);
    }
}

#[derive(Debug)]
pub struct Job {
    tasks: Vec<Task>,
}

impl Job {
    pub fn new() -> Job {
        Job {
            tasks: Vec::new(),
        }
    }

    pub fn comment(&mut self, repo: String, issue: u64, body: String) {
        self.tasks.push(Task::GithubComment {
            repo: repo,
            issue: issue,
            body: body,
        });
    }
}

#[derive(Debug)]
enum Task {
    GithubComment {
        repo: String,
        issue: u64,
        body: String,
    },
}
