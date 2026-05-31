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
    // TODO: This callback needs updating to be channelId/key, as multiple people could subscribe to the same url/key
    let mut callbacks: HashMap<String, AsyncCallback> = HashMap::new();
    let mut ticker = interval(polling_interval);
    let mut remaining_errors = errors_until_close.unwrap_or(3);

    'worker: loop {
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
                            break 'worker; // break the loop to return and cleanup the worker
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

                                // If the callback errors, add it to the count
                                if let Err(e) = cb(status).await {
                                    println!("Callback error for key {key}: {e:?}");
                                    remaining_errors -= 1;

                                    // If the errors have passed the threshold, break the loop to return and cleanup the worker
                                    if remaining_errors <= 0 {
                                        println!("Worker for host {host} has gone over the max error count. Closing loop.");
                                        break 'worker;
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        println!("Failed to get stream statuses for host {host}! Error: {:?}", e);
                        remaining_errors -= 1;

                        if remaining_errors <= 0 {
                            println!("Worker for host {host} has gone over the max error count. Closing loop.");
                            break 'worker;
                        }
                    }
                }
            }
        }
    }
}
