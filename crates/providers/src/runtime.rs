//! The one background tokio runtime (§4.7): rig is async-only, the rest of
//! Agentique is not.

use futures::future::{AbortHandle, Abortable};
use std::future::Future;
use std::sync::OnceLock;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("agq-providers")
            .enable_all()
            .build()
            .expect("the providers' runtime could not start")
    })
}

/// Runs `task` in the background; aborting the handle drops it at its next
/// await point, which closes its connections.
pub(crate) fn spawn(task: impl Future<Output = ()> + Send + 'static) -> AbortHandle {
    let (handle, registration) = AbortHandle::new_pair();
    runtime().spawn(Abortable::new(task, registration));
    handle
}

/// Runs `task` in the background and waits for its result for at most
/// `timeout`; `None` when it took longer (the task is then dropped).
pub(crate) fn run<T: Send + 'static>(
    timeout: std::time::Duration,
    task: impl Future<Output = T> + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let handle = spawn(async move {
        let _ = sender.send(task.await);
    });
    let result = receiver.recv_timeout(timeout).ok();
    handle.abort();
    result
}

/// Waits for `duration` (retry pauses).
pub(crate) async fn sleep(duration: std::time::Duration) {
    tokio::time::sleep(duration).await;
}
