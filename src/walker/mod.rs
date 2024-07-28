// Walk the source directory, and parse the directories / files
// into the destination directory.

mod route;
mod walker;

pub use route::{RouteConfig, ThemeConfig};
pub use walker::*;
