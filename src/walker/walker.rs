use std::path::PathBuf;

use anyhow::{Context, Result};

use async_recursion::async_recursion;
use tokio::task::JoinSet;
use tracing::info;

use super::{
    file::process_file,
    route::{Route, RouteConfig, RouteContext},
};

use crate::{fatal, theme::TemplateRegistry};

const ROOT: &str = "__root__.toml";

#[derive(Clone, Debug)]
pub struct Walker {
    source: PathBuf,
    destination: PathBuf,

    context: RouteContext,
}

impl Walker {
    pub fn new(source: PathBuf, destination: PathBuf, registry: TemplateRegistry) -> Self {
        Walker {
            source,
            destination,
            context: RouteContext {
                registry,
                config: RouteConfig::new(),
            },
        }
    }

    pub async fn walk(self: Self) {
        let routes = walk_directory(self).await;
        if let Err(err) = routes {
            fatal!("Error: {}", err.to_string());
        }
    }
}

#[async_recursion]
async fn walk_directory(config: Walker) -> Result<Route> {
    let Walker {
        source,
        destination,
        context,
    } = &config;

    let mut entries = tokio::fs::read_dir(&source)
        .await
        .context(format!("Could not read directory `{}`", source.display()))?;

    let mut children_tasks = JoinSet::new(); // spawn handles for all the recursive calls below

    while let Ok(entry_) = entries.next_entry().await {
        let entry = entry_.context(format!("Failed to read directory `{}`", source.display()))?;

        info!("Reading `{}`", entry.path().display());

        let ft = entry.file_type().await.context(format!(
            "Failed to read file-type for `{}`",
            entry.path().display()
        ))?;

        let name = entry.file_name();

        let mut config = config.clone();

        // spawn tasks for child routes
        if ft.is_dir() {
            config.source.push(&name);
            config.destination.push(&name);

            children_tasks.spawn(walk_directory(config));
        } else if ft.is_file() {
            children_tasks.spawn(process_file(config, name));
        };
    }

    let mut children = vec![];

    // wait on the tasks for children and add them to the current list of children routes
    while let Some(result) = children_tasks.join_next().await {
        let route = result
            .context(format!(
                "Failed to finish join for subpath `{}`!",
                source.display()
            ))?
            .context(format!(
                "Error encountered while parsing subpath `{}`!",
                source.display()
            ))?;
        children.push(route);
    }

    todo!()
}
