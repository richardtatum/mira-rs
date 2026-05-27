use std::future::Future;
use std::time::Duration;

use mira_broadcast_box::BroadcastBoxClient;
use mira_core::domain::dispatcher::Dispatcher;
use mira_core::{AsyncCallback, CoreError, StreamStatus};

pub struct StreamWatcher {
    dispatcher: Dispatcher,
}

impl StreamWatcher {
    pub fn new(host_polling_interval_secs: Option<u64>) -> Self {
        Self { dispatcher: Dispatcher::new(host_polling_interval_secs.map(Duration::from_secs)) }
    }

    pub fn watch<F, Fut>(&self, url: String, auth_header: Option<String>, key: String, f: F) -> Result<(), CoreError>
    where
        F: Fn(StreamStatus) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), CoreError>> + Send + 'static,
    {
        let callback: AsyncCallback = Box::new(move |status| Box::pin(f(status)));
        let provider = BroadcastBoxClient::new(url.clone(), auth_header)?;
        self.dispatcher.register(url, key, provider, callback)
    }
}
