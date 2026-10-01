//! Sends each notification to the gateway, and retries a failed send up to
//! `max_attempts` times, waiting `backoff_ms` longer each time.

use crate::gateway::Gateway;
use crate::model::{DeliveryStatus, Notification, Receipt, SendRequest};

/// Waits: real time in production, logical time in the harness.
pub trait Clock {
    fn sleep_ms(&mut self, ms: u64);
}

pub struct Dispatcher<G: Gateway, C: Clock> {
    pub gateway: G,
    pub clock: C,
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

impl<G: Gateway, C: Clock> Dispatcher<G, C> {
    pub fn new(gateway: G, clock: C) -> Self {
        Dispatcher {
            gateway,
            clock,
            max_attempts: 3,
            backoff_ms: 200,
        }
    }

    /// Sends a notification, retrying failures; the receipt says how it ended.
    pub fn dispatch(&mut self, notification: &Notification) -> Receipt {
        let mut attempts = 0;
        loop {
            attempts += 1;
            let request = SendRequest {
                id: notification.id.clone(),
                attempt: attempts,
            };
            let delivered = matches!(self.gateway.send(&request), Ok(result) if result.ok);
            if delivered {
                return Receipt {
                    id: notification.id.clone(),
                    status: DeliveryStatus::Delivered,
                    attempts,
                };
            }
            if attempts >= self.max_attempts {
                return Receipt {
                    id: notification.id.clone(),
                    status: DeliveryStatus::Failed,
                    attempts,
                };
            }
            self.clock.sleep_ms(self.backoff_ms * u64::from(attempts));
        }
    }
}
