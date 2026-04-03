use async_trait::async_trait;
use mira_core::StreamStatusProvider;

pub struct BroadcastBoxClient {
    base_url: String,
}

#[async_trait]
impl StreamStatusProvider for BroadcastBoxClient {
    async fn get_status(&self, key: &str) -> Result<StreamStatus, CoreError> {
        // HTTP call here
    }
}
