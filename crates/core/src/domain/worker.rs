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
    errors_until_close: Option<u32>,
) {
    let host = status_provider.get_host();
    let mut callbacks: HashMap<String, AsyncCallback> = HashMap::new();
    let mut ticker = interval(polling_interval);
    let mut remaining_errors = errors_until_close.unwrap_or(3);

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
                        if callbacks.is_empty() {
                            println!("Callbacks for {host} are now empty. Closing loop.");
                            break; // Break the loop and return, triggering a cleanup
                        }
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
                        println!("Failed to get stream statuses for host {host}! Error: {:?}", e);
                        remaining_errors -= 1;
                        if remaining_errors <= 0 {
                            println!("Worker for host {host} has gone over the max error count. Closing loop.");
                            break; // Break the loop and return
                        }
                    }
                }
            }
        }
    }
}
