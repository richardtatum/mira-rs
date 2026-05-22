use std::collections::HashMap;

use crate::models::error::CoreError;
use crate::models::persistence::{Host, Subscription};
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
        key: String,
        host_guild_id: i64,
        channel_id: i64,
        created_by: i64,
    ) -> Result<i64, CoreError>;

    async fn get_subscription(&self, subscription_id: i64) -> Result<Subscription, CoreError>;

    async fn set_subscription_message_id(
        &self,
        subscription_id: i64,
        message_id: i64,
    ) -> Result<(), CoreError>;

    async fn clear_subscription_message_id(&self, subscription_id: i64) -> Result<(), CoreError>;

    // async fn get_subscriptions(&self, host_url: String) -> Vec<Subscription>;
}
