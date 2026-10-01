//! The SMS gateway the dispatcher sends through.

use crate::model::{SendRequest, SendResult};

/// Why a send got no result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayError {
    Timeout,
    Unavailable(String),
}

/// Sends one text message.
pub trait Gateway {
    fn send(&mut self, request: &SendRequest) -> Result<SendResult, GatewayError>;
}

/// The production gateway would call the provider's API; this example has
/// none, so it reports every send as delivered.
pub struct SmsGateway;

impl Gateway for SmsGateway {
    fn send(&mut self, request: &SendRequest) -> Result<SendResult, GatewayError> {
        Ok(SendResult {
            id: request.id.clone(),
            ok: true,
        })
    }
}
