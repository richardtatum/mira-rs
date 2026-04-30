use std::collections::HashMap;

use crate::models::error::CoreError;
use crate::models::status::StreamStatus;
use async_trait::async_trait;

#[async_trait]
pub trait StreamStatusProvider: Send + Sync {
    async fn get_statuses(
        &self,
        keys: Vec<&str>,
    ) -> Result<HashMap<String, StreamStatus>, CoreError>;
}
