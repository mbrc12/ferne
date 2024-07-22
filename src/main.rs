mod theme;
mod util;
mod walker;
mod worker;

use std::path::PathBuf;

use clap::Parser;
use tokio::task::JoinSet;
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

    info!(
        "Building files from {} to {}.",
        source.display(),
        destination.display()
    );

    let (resource_worker, queue) = worker::Worker::new();

    // spawn the resource worker
    tokio::spawn(resource_worker.work(source.clone()));

    let mut joinset = JoinSet::new();

    for _ in 0..10 {
        let queue_ = queue.clone();

        joinset.spawn(async move {
            let cell = queue_.submit("https://mriganka.xyz/blog").await.unwrap();
            info!("{}", cell.get().await);
        });
    }

    while joinset.join_next().await.is_some() {}

    // spawn the directory walker
    // tokio::spawn(walker::walker_entrypoint(
    //     walker::WalkerConfig {
    //         source,
    //         destination,
    //     },
    //     queue,
    // ));

    Ok(())
}
