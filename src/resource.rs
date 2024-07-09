// use std::{future::Future, path::PathBuf, pin::Pin};
//
// use anyhow::Context;
// use tokio::sync::OnceCell;
//
// type RawData = Vec<u8>;
// type BoxedFuture<T> = Pin<Box<dyn Future<Output = Option<T>> + Send + Sync + 'static>>;
//
// #[derive(Hash, Eq, PartialEq, Clone, Debug)]
// pub enum ResourcePath {
//     Local(PathBuf),
//     Blank,
// }
//
// struct ResourceInner<T> {
//     path: ResourcePath,
//     processor: Box<dyn FnOnce(RawData) -> BoxedFuture<T>>,
//     result: OnceCell<anyhow::Result<T>>,
// }
//
// impl<T> Resource<T> {
//     pub fn new(
//         path: ResourcePath,
//         processor: impl FnOnce(RawData) -> BoxedFuture<T> + 'static,
//     ) -> Self {
//         Resource {
//             path,
//             processor: Box::new(processor),
//             result: OnceCell::new(),
//         }
//     }
//
//     pub async fn get(&self) -> &anyhow::Result<T> {
//         let Resource {
//             path,
//             processor,
//             result,
//         } = self;
//
//         // initialize the once-cell by trying to load the resource
//         // or if it is loaded and processed, just return the result
//         result
//             .get_or_init(|| async {
//                 let raw_data = fetch_bytes_of_resource(&path).await?;
//                 let processed = processor(raw_data).await;
//                 processed.context(format!("Failed to process resource {:?}", path))
//             })
//             .await
//     }
// }
//
// async fn fetch_bytes_of_resource(path: &ResourcePath) -> anyhow::Result<RawData> {
//     use ResourcePath::*;
//
//     match path {
//         Local(path) => load_local(path).await,
//         Blank => Ok(vec![]),
//     }
// }
//
// async fn load_local(path: &PathBuf) -> anyhow::Result<RawData> {
//     tokio::fs::read(path)
//         .await
//         .context(format!("Failed to load file {}", path.display()))
// }
