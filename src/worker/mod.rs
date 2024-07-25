// Responsible for caching and loading files from the web or local fs,
// and retrying upon fail. The worker is run on a separate thread
// by the runtime, as started from main. The files are returned
// as LoadResponse's, which are lazily loaded files.

mod loaders;
mod resource;
mod resource_path;
mod worker;

pub use worker::*;
