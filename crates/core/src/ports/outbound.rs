use crate::models::error::CoreError;
use crate::models::status::StreamStatus;
use async_trait::async_trait;

#[async_trait]
pub trait StreamStatusProvider: Send + Sync {
    async fn get_status(&self, key: &str) -> Result<StreamStatus, CoreError>;
}
