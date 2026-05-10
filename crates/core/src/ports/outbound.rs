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
    async fn get_hosts(&self) -> Result<Vec<Host>, CoreError>;

    // async fn get_subscriptions(&self, host_url: String) -> Vec<Subscription>;
}
