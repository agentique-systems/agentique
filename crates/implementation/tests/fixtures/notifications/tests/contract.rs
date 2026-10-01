//! Contract tests for the dispatcher (protected: an implementation task may
//! not change them).

use notifications::dispatcher::{Clock, Dispatcher};
use notifications::gateway::{Gateway, GatewayError};
use notifications::model::{DeliveryStatus, Notification, SendRequest, SendResult};

struct Counting(u64);
impl Clock for Counting {
    fn sleep_ms(&mut self, ms: u64) {
        self.0 += ms;
    }
}

struct FailFirst(u32, u32);
impl Gateway for FailFirst {
    fn send(&mut self, request: &SendRequest) -> Result<SendResult, GatewayError> {
        self.1 += 1;
        Ok(SendResult {
            id: request.id.clone(),
            ok: self.1 > self.0,
        })
    }
}

fn note() -> Notification {
    Notification {
        id: "n".into(),
        text: "t".into(),
    }
}

#[test]
fn receipts_count_attempts() {
    let mut dispatcher = Dispatcher::new(FailFirst(2, 0), Counting(0));
    let receipt = dispatcher.dispatch(&note());
    assert_eq!(receipt.status, DeliveryStatus::Delivered);
    assert_eq!(receipt.attempts, 3);
    assert_eq!(dispatcher.clock.0, 200 + 400);
}

#[test]
fn a_gateway_that_keeps_failing_ends_as_failed() {
    let mut dispatcher = Dispatcher::new(FailFirst(10, 0), Counting(0));
    let receipt = dispatcher.dispatch(&note());
    assert_eq!(receipt.status, DeliveryStatus::Failed);
    assert_eq!(receipt.attempts, 3);
    assert_eq!(dispatcher.gateway.1, 3, "never a fourth call");
}
