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

/// Waits for `duration` (retry pauses).
pub(crate) async fn sleep(duration: std::time::Duration) {
    tokio::time::sleep(duration).await;
}
