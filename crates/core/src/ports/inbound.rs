use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::models::status::StreamStatus;

pub type AsyncCallback = Box<dyn Fn(StreamStatus) -> Pin<Box<dyn Future<Output = Result<(), crate::CoreError>> + Send>> + Send + Sync>;
pub type SubscriptionToken = Uuid;
