use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::info;

use crate::theme::{register_template, ResourcePath};

async fn load_local(path: &PathBuf) -> anyhow::Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .context(format!("Failed to load file {}", path.display()))
}

pub type LoadResponse = Result<(), String>;

struct LoadTask {
    resource_path: ResourcePath,
    response_chan: oneshot::Sender<LoadResponse>,
}

pub type TemplateRegistry = Arc<RwLock<handlebars::Handlebars<'static>>>;
type FileIndex = Arc<RwLock<HashMap<ResourcePath, Arc<RwLock<bool>>>>>;

#[derive(Clone)]
pub struct DB {
    hb_registry: TemplateRegistry,
    files: FileIndex, // is this file loaded
}

pub struct Worker {
    db: DB,
    ingest_queue: mpsc::Receiver<LoadTask>,
}

#[derive(Clone)]
pub struct SubmitQueue(mpsc::Sender<LoadTask>);

impl SubmitQueue {
    pub async fn submit(self, resource_path: ResourcePath) -> LoadResponse {
        info!("Queueing fetch for {}", resource_path);

        let (send, recv) = oneshot::channel::<LoadResponse>();
        let task = LoadTask {
            resource_path,
            response_chan: send,
        };

        if let Err(_) = self.0.send(task).await {
            return Err("Failed to send task to worker".to_string());
        }

        let response = recv.await;

        if let Err(_) = response {
            return Err("Failed to receive response from resource worker.".to_string());
        } else {
            return response.unwrap();
        }
    }
}

impl Worker {
    pub fn new() -> (Self, SubmitQueue) {
        let (submit_queue, ingest_queue) = mpsc::channel(16);

        (
            Worker {
                db: DB {
                    hb_registry: Arc::new(RwLock::new(handlebars::Handlebars::new())),
                    files: Arc::new(RwLock::new(HashMap::new())),
                },
                ingest_queue,
            },
            SubmitQueue(submit_queue),
        )
    }

    pub fn get_db(&self) -> TemplateRegistry {
        return self.db.hb_registry.clone();
    }

    pub async fn work(self) {
        let Worker {
            db,
            mut ingest_queue,
        } = self;

        loop {
            let LoadTask {
                resource_path,
                response_chan,
            } = {
                let task = ingest_queue.recv().await;
                if task.is_none() {
                    // channel closed, no more work left
                    return;
                }
                task.unwrap()
            };

            let db_clone = db.clone();

            tokio::spawn(async {
                let response = process_single(resource_path, db_clone).await;
                let _ = response_chan.send(response); // error caught on other side
            });
        }
    }
}

async fn process_single(resource_path: ResourcePath, db: DB) -> LoadResponse {
    async fn load(
        path: ResourcePath,
        registry: TemplateRegistry,
        status: Arc<RwLock<bool>>,
    ) -> LoadResponse {
        // lock the status since you are loading the resource
        let mut status_locked = status.write().await;

        use ResourcePath::*;

        // delegate loading the resource to the appropriate function
        let data = match &path {
            Local(path) => load_local(&path).await,
            URL(_url) => unimplemented!("URL downloads not implemented yet!"),
            GitHub(_url) => unimplemented!("Github downloads not implemented yet!"),
        };

        if let Err(_) = data {
            return Err(format!("Failed to load path {:?}", path));
        }

        // load the data into the template repository
        let response = register_template(data.unwrap(), registry).await;

        *status_locked = true; // indicate that this is done

        return response;
    }

    let files_read = db.files.read().await;

    // first check if the path is registered
    if let Some(status) = files_read.get(&resource_path) {
        // if registered, acquire the lock on the mutex, and block if its held elsewhere
        let status_lock = status.read().await;
        if *status_lock == false {
            // resource not loaded yet
            drop(status_lock); // release the read lock
            load(resource_path, db.hb_registry, status.clone()).await
        } else {
            // otherwise the resource is already loaded, nothing to do
            Ok(())
        }
    } else {
        // if it is not registered

        drop(files_read); // explicitly release the read lock

        // reacquire a lock to write to the list of files
        let mut files_write = db.files.write().await;

        // the lock indicating that the current resource
        // is being fetched right now by some thread
        let status = Arc::new(RwLock::new(false));

        // create the status lock and insert it
        files_write.insert(resource_path.clone(), status.clone());

        drop(files_write); // explicitly release the write lock

        // queue an attempt to load the resource
        load(resource_path, db.hb_registry, status).await
    }
}
