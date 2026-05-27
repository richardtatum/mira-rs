use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

use crate::domain::worker::poll_endpoint;
use crate::models::command::Command;
use crate::ports::inbound::AsyncCallback;
use crate::{CoreError, StreamStatusProvider};

pub struct Dispatcher {
    // Dashmap is essentially a ConcurrentHashMap
    workers: Arc<DashMap<String, mpsc::UnboundedSender<Command>>>,
    interval: Duration,
}

impl Dispatcher {
    pub fn new(polling_interval: Option<Duration>) -> Self {
        Self { workers: Arc::new(DashMap::new()), interval: polling_interval.unwrap_or(Duration::from_secs(30)) }
    }

    pub fn register<P: StreamStatusProvider + 'static>(
        &self,
        url: String,
        key: String,
        provider: P,
        callback: AsyncCallback,
    ) -> Result<(), CoreError> {
        // See if there is already a worker for this url, otherwise create a new one
        let interval = self.interval;
        let sender = self.workers.entry(url.clone()).or_insert_with(|| {
            let (tx, rx) = mpsc::unbounded_channel();

            let handle = tokio::spawn(poll_endpoint(rx, interval, provider, None));

            // Pass the handle and create a task that removes the entry when the function returns
            // Function returns only happens when all keys are empty after a remove
            self.spawn_cleanup_task(handle, url.clone());

            tx
        });

        // Register the key with the worker via the message channel
        sender.send(Command::AddKey(key, callback)).map_err(|e| CoreError::StreamError(e.to_string()))?;

        Ok(())
    }

    pub fn deregister(&mut self, url: String, key: String) -> Result<(), CoreError> {
        if let Some(sender) = self.workers.get(&url) {
            sender.send(Command::RemoveKey(key)).map_err(|e| CoreError::StreamError(e.to_string()))?;
            return Ok(());
        }

        // Worker no longer exists, just quietly exit
        println!("No worker is registered for url '{}', so can't deregister key '{}'. Ignoring request", url, key);
        Ok(())
    }

    fn spawn_cleanup_task(&self, handle: JoinHandle<()>, url: String) {
        let workers = Arc::clone(&self.workers);
        tokio::spawn(async move {
            let _ = handle.await; // This returns when the 'poll_endpoint' loop is broken
            workers.remove(&url);
        });
    }
}
