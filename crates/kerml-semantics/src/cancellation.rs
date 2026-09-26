//! Explicit operational cancellation, independent of semantic acceptance.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// A request to stop unpublished work. Cancellation never denotes an incomplete
/// semantic answer or grants authority to retain a partial computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancelled;
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("operation cancelled")
    }
}
impl std::error::Error for Cancelled {}

/// One operation's shared cancellation request. Allocate a fresh token for every
/// operation; an existing token cannot be reset or reused for a later request.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Request cancellation. Only a terminal worker outcome acknowledges it.
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Observe the operation-local request without affecting semantic state.
    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Stop at a cooperative boundary, returning no partial semantic result.
    pub fn check(&self) -> Result<(), Cancelled> {
        if self.is_requested() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_shared_with_only_the_same_operation() {
        let first = CancellationToken::default();
        let worker = first.clone();
        let next = CancellationToken::default();
        first.request();
        assert_eq!(worker.check(), Err(Cancelled));
        assert_eq!(next.check(), Ok(()));
        assert_eq!(first.check(), Err(Cancelled));
    }
}
