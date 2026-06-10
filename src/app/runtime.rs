use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::Runtime;
use tokio::sync::oneshot;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// The process-wide tokio runtime (lazily built on first use).
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
    })
}

/// Spawns `fut` on the tokio runtime and returns a receiver for its result.
///
/// The receiver is a `Future`, so a view awaits it inside `cx.spawn(async move
/// |this, cx| { if let Ok(v) = rx.await { ... } })`.
pub fn spawn<T, F>(fut: F) -> oneshot::Receiver<T>
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    runtime().spawn(async move {
        let _ = tx.send(fut.await);
    });
    rx
}

/// Fire-and-forget variant for backend work whose result the UI doesn't need.
pub fn detach<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    runtime().spawn(fut);
}
