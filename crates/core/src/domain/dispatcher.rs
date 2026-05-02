use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::StreamStatusProvider;
use crate::domain::worker::poll_endpoint;
use crate::models::command::Command;
use crate::ports::inbound::AsyncCallback;

pub struct Dispatcher {
    workers: HashMap<String, mpsc::UnboundedSender<Command>>,
    interval: Duration,
}

impl Dispatcher {
    pub fn new(polling_interval: Option<Duration>) -> Self {
        Self {
            workers: HashMap::new(),
            interval: polling_interval.unwrap_or(Duration::from_secs(30)),
        }
    }

    pub fn register<P: StreamStatusProvider + 'static>(
        &mut self,
        url: String,
        key: String,
        provider: P,
        callback: AsyncCallback,
    ) {
        // See if there is already a worker for this url, otherwise create a new one
        // and register this key
        let interval = self.interval;
        let sender = self.workers.entry(url.clone()).or_insert_with(|| {
            let (tx, rx) = mpsc::unbounded_channel();

            tokio::spawn(poll_endpoint(rx, interval, provider));

            tx
        });

        sender.send(Command::AddKey(key, callback)).unwrap();
    }

    pub fn deregister<P: StreamStatusProvider + 'static>(&mut self, url: String, key: String) {
        // TODO: We need to handle the potential that all keys have been removed and we can remove it from the workers map
        if let Some(sender) = self.workers.get(&url) {
            sender.send(Command::RemoveKey(key)).unwrap();
        }
    }
}
