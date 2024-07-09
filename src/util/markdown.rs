use std::path::PathBuf;

use anyhow::Context;

pub async fn read(path: &PathBuf) -> anyhow::Result<String> {
    let path_str = path.display();

    let contents = tokio::fs::read_to_string(path)
        .await
        .context(format!("File {} was required but not found.", path_str))?;

    Ok(markdown::to_html(&contents))
}
