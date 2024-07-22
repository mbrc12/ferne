use std::{future::Future, pin::Pin, sync::Arc};

use tokio::sync::OnceCell;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

// A cell that contains a loader function that loads exactly once.
// Thanks to Alice Ryhl on the tokio discord for this pattern.
#[derive(Clone)]
pub struct Resource<T> {
    inner: Arc<ResourceInner<T>>,
}

struct ResourceInner<T> {
    // the function producing the boxed future
    f: Box<dyn Fn() -> BoxFuture<T> + Send + Sync + 'static>,

    // the cell caching the output of f().await
    cell: OnceCell<T>,
}

impl<T> Resource<T> {
    pub fn new(f: impl Fn() -> BoxFuture<T> + Send + Sync + 'static) -> Resource<T> {
        Resource {
            inner: Arc::new(ResourceInner {
                f: Box::new(f),
                cell: OnceCell::new(),
            }),
        }
    }

    pub async fn get(self: &Self) -> &T {
        self.inner
            .cell
            .get_or_init(|| async { (self.inner.clone().f)().await })
            .await
    }
}
