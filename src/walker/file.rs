use std::ffi::OsString;

use anyhow::Result;

use async_recursion::async_recursion;

use super::{route::Route, Walker};

#[async_recursion]
pub async fn process_file(config: Walker, name: OsString) -> Result<Route> {
    dbg!(config);
    todo!()
}
