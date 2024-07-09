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
    #[arg(short, long, default_value = ".")]
    source: String,

    #[arg(short, long, default_value = "./build")]
    destination: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let CLIArguments {
        source,
        destination,
    } = CLIArguments::parse();

    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);

    let (resource_worker, queue) = worker::Worker::new();
    let registry = resource_worker.get_db();

    // spawn the resource worker
    tokio::spawn(resource_worker.work());

    info!(
        "Building files from {} to {}.",
        source.display(),
        destination.display()
    );

    Ok(())
}
