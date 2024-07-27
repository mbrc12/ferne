use std::path::PathBuf;

use crate::{fatal, fatal_if_err};

// Remove the already existing directory, if necessary (and force),
// and then create a new directory in its place
pub async fn remove_and_create(path: &PathBuf, force: bool) {
    let path_disp = path.display();

    let exists = fatal_if_err!(tokio::fs::try_exists(path).await;
        "Error reading path `{}`.", path_disp);

    if exists {
        if !force {
            fatal!(
                "Directory `{}` already exists. Use --force to force deletion.",
                path_disp
            );
        } else {
            fatal_if_err!(tokio::fs::remove_dir_all(path).await;
                "Failed to delete directory `{}`!", path_disp);
        }
    }

    // Directory does not exist at this point

    fatal_if_err!(tokio::fs::create_dir(path).await;
        "Failed to create directory `{}`!", path_disp);
}
