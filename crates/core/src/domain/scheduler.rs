use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::StreamStatusProvider;
use crate::domain::worker::endpoint_worker;
use crate::models::command::Command;
use crate::ports::inbound::AsyncCallback;

pub struct Scheduler {
    workers: HashMap<String, mpsc::UnboundedSender<Command>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    pub fn register<P: StreamStatusProvider + 'static>(
        &mut self,
        url: String,
        key: String,
        interval: Duration,
        provider: P,
        callback: AsyncCallback,
    ) {
        // See if there is already a worker for this url, otherwise create a new one
        // and register this key
        let sender = self.workers.entry(url.clone()).or_insert_with(|| {
            let (tx, rx) = mpsc::unbounded_channel();

            tokio::spawn(endpoint_worker(rx, interval, provider));

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
