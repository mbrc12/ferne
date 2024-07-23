use std::{future::Future, pin::Pin, sync::Arc};

use tokio::sync::OnceCell;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

// A cell that contains a loader function that loads exactly once.
// Effectively, this is a lazy resource that loads exactly once with the given loader.
// A similar idiom is in the crate async-once-cell::Lazy, but I do not understand
// it enough to use it.
// Thanks to Alice Ryhl on the tokio discord for suggesting this pattern
// with Box<dyn Fn() -> BoxFuture> ...
#[derive(Clone)]
pub struct Resource<T> {
    inner: Arc<Inner<T>>,
}

struct Inner<T> {
    // the function producing the boxed future
    f: Box<dyn Fn() -> BoxFuture<T> + Send + Sync + 'static>,

    // the cell caching the output of f().await
    cell: OnceCell<T>,
}

impl<T> Resource<T> {
    pub fn new(f: impl Fn() -> BoxFuture<T> + Send + Sync + 'static) -> Resource<T> {
        Resource {
            inner: Arc::new(Inner {
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
