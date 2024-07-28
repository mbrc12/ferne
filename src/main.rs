mod theme;
mod util;
mod walker;
mod worker;

use std::path::PathBuf;

use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct CLIArguments {
    #[arg(short, long, default_value = "./src")]
    source: String,

    #[arg(short, long, default_value = "./build")]
    destination: String,

    #[arg(short, long, default_value_t = false)]
    force: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let CLIArguments {
        source,
        destination,
        force,
    } = CLIArguments::parse();

    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);

    info!(
        "Building files from `{}` to `{}`.",
        source.display(),
        destination.display()
    );

    if force {
        info!("Deleting folders while rebuilding due to --force flag set.");
    }

    let (resource_worker, queue) = worker::Worker::new(source.clone());
    tokio::spawn(resource_worker.work());

    let template_registry = theme::TemplateRegistry::new(queue.clone())?;

    walker::Walker::new(source, destination, force, template_registry)
        .walk()
        .await;

    Ok(())
}
