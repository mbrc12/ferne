use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result};

use async_recursion::async_recursion;
use tokio::task::JoinSet;
use tracing::info;

use super::route::{Route, RouteConfig, RouteContext, RouteDetails};

use crate::{fatal, fatal_if_err, theme::TemplateRegistry, util, walker::route::DirectoryRoute};

const COMMON_CONFIG_FILE: &str = "common.toml";

#[derive(Clone, Debug)]
pub struct Walker {
    source: PathBuf,
    destination: PathBuf,
    force: bool,

    context: RouteContext,
}

impl Walker {
    pub fn new(
        source: PathBuf,
        destination: PathBuf,
        force: bool,
        registry: TemplateRegistry,
    ) -> Self {
        Walker {
            source,
            destination,
            force,
            context: RouteContext {
                registry,
                config: RouteConfig::default(),
            },
        }
    }

    pub async fn walk(self: Self) {
        // Setup the destination directory
        let dest_display = self.destination.display();

        let exists = fatal_if_err!(tokio::fs::try_exists(&self.destination).await;
            "Failed to check directory {}", dest_display);
        if exists {
            if !self.force {
                fatal!("Directory {} exists! Use the option `--force` to delete the directory and use it as target.", 
                    dest_display);
            } else {
                fatal_if_err!(tokio::fs::remove_dir(&self.destination).await; 
                    "Failed to delete directory {}!", dest_display);
            }
        }

        fatal_if_err!(tokio::fs::create_dir(&self.destination).await;
            "Failed to create directory {}", dest_display);

        // Walk the source
        let routes = process_directory(self).await;
        if let Err(err) = routes {
            fatal!("Error: {}", err.to_string());
        }
    }
}

// Uses option to match the type of process_file below
#[async_recursion]
async fn process_directory(config: Walker) -> Result<Option<Route>> {
    let Walker {
        source,
        destination,
        context,
        ..
    } = &config;

    let mut entries = tokio::fs::read_dir(&source)
        .await
        .context(format!("Could not read directory `{}`", source.display()))?;

    // Produce the directory in the destination
    tokio::fs::create_dir(destination).await?;

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

            children_tasks.spawn(process_directory(config));
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

        if let Some(route_) = route {
            children.push(route_);
        }
    }

    let route_details = RouteDetails::Dir(DirectoryRoute { children });

    let common_toml = {
        let mut common_toml_path = source.clone();
        common_toml_path.push(COMMON_CONFIG_FILE);
        util::toml::read(&common_toml_path).await
    }?;

    let route_config = context.clone().route_config_from_toml(common_toml).await?;

    Ok(Some(Route {
        config: route_config,
        details: route_details,
    }))
}

// check if extension matches
fn ext_is(val: &PathBuf, ext: &str) -> bool {
    if let Some(val_) = val.extension() {
        if val_.to_string_lossy().eq(ext) {
            true
        } else {
            false
        }
    } else {
        false
    }
}

// Returns Ok(None) if the path should be ignored currently (for example
// if it is a .toml file). Currently ignores everything that is not a .md
#[async_recursion]
pub async fn process_file(config: Walker, name: OsString) -> Result<Option<Route>> {
    let name = PathBuf::from(name);

    if !ext_is(&name, "md") {
        return Ok(None) // do not process this file
    }

    let stem = name
        .file_stem()
        .context(format!("File name cannot be parsed!"))?
        .to_string_lossy();

    let file_config = {
        let mut path = config.source.clone();
        path.push(format!("{}.toml", stem));
        util::toml::read(&path).await?
    };

    let content = {
        let mut path = config.source.clone();
        path.push(name);
        util::markdown::read(&path).await?
    };

    todo!()
}
