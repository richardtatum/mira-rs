use std::future::Future;
use std::time::Duration;

use mira_broadcast_box::BroadcastBoxClient;
use mira_core::domain::dispatcher::Dispatcher;
use mira_core::{AsyncCallback, StreamStatus};

pub struct StreamMonitor {
    dispatcher: Dispatcher,
}

impl StreamMonitor {
    pub fn new(host_polling_interval_secs: Option<u64>) -> Self {
        Self {
            dispatcher: Dispatcher::new(host_polling_interval_secs.map(Duration::from_secs)),
        }
    }

    pub fn register_stream<F, Fut>(
        &self,
        url: String,
        auth_token: Option<String>,
        key: String,
        f: F,
    ) where
        F: Fn(StreamStatus) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let callback: AsyncCallback = Box::new(move |status| Box::pin(f(status)));
        let provider = BroadcastBoxClient::new(url.clone(), auth_token);
        self.dispatcher.register(url, key, provider, callback);
    }
}
