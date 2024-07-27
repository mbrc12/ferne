use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result};

use async_recursion::async_recursion;
use tokio::task::JoinSet;
use tracing::info;

use super::route::{Route, RouteConfig, RouteContext, RouteDetails};

use crate::{fatal, fatal_if_err, theme::TemplateRegistry, util, walker::route::DirectoryRoute};

const COMMON_CONFIG_FILE: &str = "__common.toml";

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
        util::dir::remove_and_create(&self.destination, self.force).await;

        // Walk the source
        let routes = process_directory(self).await;
        if let Err(err) = routes {
            fatal!("Error: {}", err.to_string());
        }

        dbg!(routes.unwrap());
    }
}

// Uses option to match the type of process_file below
#[async_recursion]
async fn process_directory(mut walker: Walker) -> Result<Option<Route>> {
    let common_toml = {
        let mut common_toml_path = walker.source.clone();
        common_toml_path.push(COMMON_CONFIG_FILE);
        info!(
            "Found common configuration file: `{}`",
            common_toml_path.display()
        );
        util::toml::read(&common_toml_path).await
    }?;

    // update context with common toml
    let context = walker.context.clone().merge_toml(common_toml).await?;
    walker.context = context;

    let Walker {
        source,
        destination,
        force,
        ..
    } = &walker;

    // Create destination directory
    util::dir::remove_and_create(destination, *force).await;

    let mut entries = tokio::fs::read_dir(&source)
        .await
        .context(format!("Could not read directory `{}`", source.display()))?;

    let mut children_tasks = JoinSet::new(); // spawn handles for all the recursive calls below

    loop {
        let entry = {
            let entry = entries
                .next_entry()
                .await
                .context(format!("Failed to read directory `{}`", source.display()))?;
            if let Some(entry_) = entry {
                entry_
            } else {
                // if option is none, then we have finished reading the directory
                break;
            }
        };

        let ft = entry.file_type().await.context(format!(
            "Failed to read file-type for `{}`",
            entry.path().display()
        ))?;

        let path = entry.path();
        let disp = path.display();

        let name = entry.file_name();

        let mut config = walker.clone();

        // spawn tasks for child routes
        if ft.is_dir() {
            config.source.push(&name);
            config.destination.push(&name);

            info!("Reading directory `{}`", disp);

            // Recursively read directory
            children_tasks.spawn(process_directory(config));
        } else if ft.is_file() && ext_is(&path, "md") {
            // only run for .md files
            info!("Reading file `{}`", disp);
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

    Ok(Some(Route {
        config: walker.context.config,
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

    let stem = name
        .file_stem()
        .context(format!("File name cannot be parsed!"))?
        .to_string_lossy()
        .into_owned();

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

    // Update old context with new config
    let context = config.context.merge_toml(file_config).await?;

    // use new context to produce the route
    let route = context.file_route_from_content(content).await?;

    let dest_path = {
        let mut path = config.destination.clone();
        path.push(format!("{}.html", stem));
        path
    };

    // Write to file
    fatal_if_err!(tokio::fs::write(&dest_path, &route.html).await;
        "Failed to write to path `{}`.", dest_path.display());

    Ok(Some(Route {
        config: context.config,
        details: RouteDetails::File(route),
    }))
}
