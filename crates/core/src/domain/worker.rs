use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

use crate::StreamStatusProvider;
use crate::models::command::Command;
use crate::ports::inbound::AsyncCallback;

// Worker for a single host, e.g. b.siobud.com
pub async fn poll_endpoint<P: StreamStatusProvider>(
    mut rx: mpsc::UnboundedReceiver<Command>,
    polling_interval: Duration,
    status_provider: P,
) {
    let mut callbacks: HashMap<String, AsyncCallback> = HashMap::new();
    let mut ticker = interval(polling_interval);

    loop {
        tokio::select! {
            // Process any new messages on every loop
            Some(cmd) = rx.recv() => {
                match cmd {
                    Command::AddKey(key, callback) => {
                        callbacks.insert(key, callback);
                    },
                    Command::RemoveKey(key) => {
                        callbacks.remove_entry(&key);
                    },
                }
            }

            _ = ticker.tick() => {
                if callbacks.is_empty() {
                    continue;
                }

                // Collect all the keys
                let keys: Vec<&str> = callbacks.keys().map(|k| k.as_str()).collect();

                match status_provider.get_statuses(keys).await {
                    Ok(mut statuses) => {
                        // Loop through the callbacks and match the status to provide
                        for (key, cb) in &callbacks {
                            if let Some(status) = statuses.remove(key) {
                                cb(status).await;
                            }
                        }
                    },
                    Err(e) => {
                        // TODO: We should consider exiting the loop after X number of errors, but it introduces
                        // similar issues to exiting on no keys, as we have to notify the caller in some way
                        println!("Failed to get requested stream status! Error: {:?}", e)
                    }
                }
            }
        }
    }
}
