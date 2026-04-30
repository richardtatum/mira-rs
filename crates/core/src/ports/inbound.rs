use std::future::Future;
use std::pin::Pin;

use crate::models::status::StreamStatus;

pub type AsyncCallback =
    Box<dyn Fn(StreamStatus) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
