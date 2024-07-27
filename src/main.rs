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
        "Building files from {} to {}.",
        source.display(),
        destination.display()
    );

    let (resource_worker, queue) = worker::Worker::new(source.clone());
    tokio::spawn(resource_worker.work());

    let template_registry = theme::TemplateRegistry::new(queue.clone())?;

    walker::Walker::new(source, destination, force, template_registry)
        .walk()
        .await;

    // spawn the resource worker

    // let mut joinset = JoinSet::new();
    //
    // for _ in 0..3 {
    //     let queue_ = queue.clone();
    //
    //     joinset.spawn(async move {
    //         let cell = queue_.submit("https://google.com/xyzw").await.unwrap();
    //         info!("{}", cell.get().await);
    //     });
    // }
    //
    // for _ in 0..3 {
    //     let queue_ = queue.clone();
    //
    //     joinset.spawn(async move {
    //         let cell = queue_.submit("index.md").await.unwrap();
    //         info!("{}", cell.get().await);
    //     });
    // }
    //
    // while joinset.join_next().await.is_some() {}

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
