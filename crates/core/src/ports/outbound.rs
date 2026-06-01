use std::collections::HashMap;

use crate::models::error::CoreError;
use crate::models::persistence::{Host, StreamState, SubscriptionRestore};
use crate::models::status::StreamStatus;
use async_trait::async_trait;
use uuid::Uuid;

// Handles accessing the external API, e.g. broadcast box
#[async_trait]
pub trait StreamStatusProvider: Send + Sync {
    fn get_host(&self) -> &str;

    async fn get_statuses(&self, keys: Vec<&str>) -> Result<HashMap<String, StreamStatus>, CoreError>;
}

// Handles persisting data between messages, e.g via sqlite
#[async_trait]
pub trait PersistenceProvider: Send + Sync {
    async fn add_host(&self, url: String, auth_header: Option<String>, created_by: i64) -> Result<i64, CoreError>;

    async fn get_hosts(&self, guild_id: i64) -> Result<Vec<Host>, CoreError>;

    async fn add_subscription(
        &self,
        key: String,
        host_guild_id: i64,
        channel_id: i64,
        created_by: i64,
    ) -> Result<i64, CoreError>;

    async fn get_stream_state(&self, subscription_id: i64) -> Result<StreamState, CoreError>;

    async fn mark_subscription_online(&self, subscription_id: i64, message_id: i64) -> Result<(), CoreError>;

    async fn mark_subscription_offline(&self, subscription_id: i64) -> Result<(), CoreError>;

    async fn get_all_subscriptions(&self) -> Result<Vec<SubscriptionRestore>, CoreError>;

    async fn update_subscription_token(&self, subscription_id: i64, token: Uuid) -> Result<(), CoreError>;
}
