use std::future::Future;
use std::time::Duration;

use broadcast_box::BroadcastBoxClient;
use mira_core::domain::scheduler::Scheduler;
use mira_core::{AsyncCallback, StreamStatus};

pub struct Watcher {
    scheduler: Scheduler,
}

impl Watcher {
    pub fn new(host_polling_interval_secs: Option<u64>) -> Self {
        Self {
            scheduler: Scheduler::new(host_polling_interval_secs.map(Duration::from_secs)),
        }
    }

    pub fn register_stream<F, Fut>(
        &mut self,
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
        self.scheduler.register(url, key, provider, callback);
    }
}
