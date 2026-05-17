use std::collections::HashMap;

use crate::models::error::CoreError;
use crate::models::persistence::Host;
use crate::models::status::StreamStatus;
use async_trait::async_trait;

#[async_trait]
pub trait StreamStatusProvider: Send + Sync {
    fn get_host(&self) -> &str;

    async fn get_statuses(
        &self,
        keys: Vec<&str>,
    ) -> Result<HashMap<String, StreamStatus>, CoreError>;
}

// These traits will need result types
#[async_trait]
pub trait PersistenceProvider: Send + Sync {
    async fn get_hosts(&self, guild_id: i64) -> Result<Vec<Host>, CoreError>;

    async fn add_subscription(
        &self,
        host_id: i64,
        key: String,
        channel_id: i64,
        created_by: i64,
    ) -> Result<i64, CoreError>;

    async fn add_stream(
        &self,
        subscription_id: i64,
        status: StreamStatus,
        viewer_count: i64,
        message_id: i64,
        start_time: String,
    ) -> Result<i64, CoreError>;

    // async fn get_subscriptions(&self, host_url: String) -> Vec<Subscription>;
}
