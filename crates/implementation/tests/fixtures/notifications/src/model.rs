//! The items that cross the service's ports.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryStatus {
    Delivered,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendRequest {
    pub id: String,
    pub attempt: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendResult {
    pub id: String,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub id: String,
    pub status: DeliveryStatus,
    pub attempts: u32,
}
