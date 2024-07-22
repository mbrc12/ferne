use std::path::PathBuf;

use tracing::info;

use crate::worker::{SubmitQueue, TemplateRegistry};

pub struct WalkerConfig {
    pub source: PathBuf,
    pub destination: PathBuf,
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

pub async fn walker_entrypoint(
    config: WalkerConfig,
    queue: SubmitQueue,
    registry: TemplateRegistry,
) {
    info!("Loading walker...");
    let route_tree = walk(config, queue, registry).await;
    todo!()
}

pub async fn walk(config: WalkerConfig, queue: SubmitQueue, registry: TemplateRegistry) -> Route {
    todo!()
}
