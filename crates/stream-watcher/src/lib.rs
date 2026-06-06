use std::future::Future;
use std::time::Duration;

use mira_broadcast_box::BroadcastBoxClient;
use mira_core::domain::dispatcher::Dispatcher;
use mira_core::{AsyncCallback, CoreError, StreamStatus, StreamStatusProvider, SubscriptionToken};

pub struct StreamWatcher {
    dispatcher: Dispatcher,
}

impl StreamWatcher {
    pub fn new(host_polling_interval_secs: Option<u64>) -> Self {
        Self { dispatcher: Dispatcher::new(host_polling_interval_secs.map(Duration::from_secs)) }
    }

    pub fn watch<F, Fut>(
        &self,
        url: String,
        auth_header: Option<String>,
        key: String,
        f: F,
    ) -> Result<SubscriptionToken, CoreError>
    where
        F: Fn(StreamStatus) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), CoreError>> + Send + 'static,
    {
        let callback: AsyncCallback = Box::new(move |status| Box::pin(f(status)));
        let provider = BroadcastBoxClient::new(url.clone(), auth_header.clone())?;
        self.dispatcher.register(url, auth_header, key, provider, callback)
    }

    pub fn unwatch(&self, url: String, auth_header: Option<String>, token: SubscriptionToken) -> Result<(), CoreError> {
        self.dispatcher.deregister(url, auth_header, token)
    }

    pub async fn test_host(&self, url: String, auth_header: Option<String>) -> Result<(), CoreError> {
        let provider = BroadcastBoxClient::new(url.clone(), auth_header)?;
        println!("Getting statuses for url: {}", url.clone());
        provider.get_statuses(vec![]).await?;
        Ok(())
    }
}
