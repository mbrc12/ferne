use std::path::PathBuf;

use super::resource_path::ResourcePath;
use anyhow::Context;
use tracing::warn;

const MAX_TRIES: u32 = 5;

macro_rules! loop_load {
    {$path: expr; $s: expr} => {{
        for _ in 0..MAX_TRIES {
            let result = $s;
            if let Ok(item) = result {
                return item;
            } else {
                warn!("Failed to retrieve item {}, {:?}", $path, result);
            }
        }
        panic!("Could not retrive {} after {MAX_TRIES} tries. Quitting.", $path);
    }};
}

pub async fn load_any(source: PathBuf, path: String) -> String {
    let resource_path = path.into();
    let description = format!("{}", resource_path);

    use ResourcePath::*;
    match resource_path {
        Local(path) => loop_load! { description; load_local(source.clone(), &path).await },
        URL(url) => loop_load! { description; load_url(&url).await },
    }
}

async fn load_local(mut source: PathBuf, path: &PathBuf) -> anyhow::Result<String> {
    source.push(path);
    tokio::fs::read_to_string(source)
        .await
        .context(format!("Failed to load file {}", path.display()))
}

async fn load_url(url: &str) -> anyhow::Result<String> {
    reqwest::get(url)
        .await
        .context("Failed to get resource!")?
        .text()
        .await
        .context("Failed to read text from response")
}
