use std::path::PathBuf;

use async_recursion::async_recursion;
use tokio::task::JoinSet;
use tracing::info;

use super::file::process_file;

use crate::{fatal_if_err, theme::TemplateRegistry, worker::SubmitQueue};

#[derive(Clone, Debug)]
pub struct Walker {
    source: PathBuf,
    destination: PathBuf,

    queue: SubmitQueue,
    registry: TemplateRegistry,
}

impl Walker {
    pub fn new(source: PathBuf, destination: PathBuf, queue: SubmitQueue, registry: TemplateRegistry) -> Self {
        Walker {
            source,
            destination,
            queue,
            registry
        }
    }

    pub async fn walk(self: Self) {
        walk(self).await // delegated to walk function below
    }
}

pub struct Route {
    output_path: PathBuf,

    markdown: String,
    html: String,

    theme_name: String,
    partial_id: String,

    config: toml::Table,
    extra: toml::Table,

    children: Vec<Route>,
}

async fn walk(config: Walker) {
    let route_tree = walk_directory(config).await;
    todo!()
}

#[async_recursion]
async fn walk_directory(
    config: Walker
) -> Route {
    let Walker {
        source,
        destination,
        queue,
        registry
    } = &config;

    let mut entries = fatal_if_err!(tokio::fs::read_dir(&source).await; 
            "Could not read path `{}`", source.display());

    let mut children = JoinSet::new(); // spawn handles for all the recursive calls below

    while let Ok(Some(entry)) = entries.next_entry().await {
        info!("Reading `{}`", entry.path().display());

        let ft = fatal_if_err!(entry.file_type().await; 
            "Failed to read file-type for `{}`", entry.path().display());

        let name = entry.file_name();

        let config = config.clone();

        children.spawn(async move {
            if ft.is_dir() {
                walk_directory(config).await;
            } else if ft.is_file() {
                process_file(config).await;
            }
        });
    }

    while let Some(result) = children.join_next().await {
        let route = fatal_if_err!(result; "Failed to finish join for a subpath!");
    }

    todo!()
}
