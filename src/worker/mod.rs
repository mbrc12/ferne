mod loaders;
mod resource;
mod resource_path;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::info;

use resource::Resource;

pub type LoadResponse = Resource<String>;

struct LoadTask {
    path: String,
    chan: oneshot::Sender<LoadResponse>,
}

pub type TemplateRegistry = Arc<RwLock<handlebars::Handlebars<'static>>>;
type FileIndex = Arc<RwLock<HashMap<String, Resource<String>>>>;

pub struct Worker {
    queue: mpsc::Receiver<LoadTask>,
    files: FileIndex,
}

#[derive(Clone)]
pub struct SubmitQueue(mpsc::Sender<LoadTask>);

impl SubmitQueue {
    pub async fn submit<T: ToString>(self: Self, path: T) -> anyhow::Result<LoadResponse> {
        let path = path.to_string();

        info!("Queueing fetch for {}", path);

        let (send, recv) = oneshot::channel::<LoadResponse>();
        let task = LoadTask { path, chan: send };

        self.0
            .send(task)
            .await
            .context("Failed to send task to worker!")?;

        recv.await
            .context("Failed to receive response from worker!")
    }
}

impl Worker {
    pub fn new() -> (Self, SubmitQueue) {
        let (submit_queue, ingest_queue) = mpsc::channel(16);

        (
            Worker {
                queue: ingest_queue,
                files: Arc::new(RwLock::new(HashMap::new())),
            },
            SubmitQueue(submit_queue),
        )
    }

    pub async fn work(self, source: PathBuf) {
        let Worker { mut queue, files } = self;

        loop {
            let LoadTask { path, chan } = {
                let task = queue.recv().await;
                if task.is_none() {
                    // channel closed, no more work left
                    return;
                }
                task.unwrap()
            };

            let files_ = files.clone();
            let source_ = source.clone();

            tokio::spawn(async {
                let response = process_single(source_, path, files_).await;
                let _ = chan.send(response); // error caught on other side
            });
        }
    }
}

async fn process_single(source: PathBuf, path: String, files: FileIndex) -> LoadResponse {
    let cell = {
        let files_read = files.read().await;

        if let Some(cell) = files_read.get(&path) {
            cell.clone()
        } else {
            drop(files_read); // release the read the lock

            let path_ = path.clone();

            let cell = Resource::new(move || {
                let path__ = path_.clone();
                let source_ = source.clone();
                Box::pin(async { loaders::load_any(source_, path__).await })
            });

            let mut files_write = files.write().await;
            files_write.insert(path, cell.clone());
            drop(files_write);

            cell
        }

        // otherwise files_read is dropped here anyway
    };

    cell
}
