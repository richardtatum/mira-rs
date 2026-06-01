use std::collections::HashMap;

use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

use crate::StreamStatusProvider;
use crate::models::command::Command;
use crate::ports::inbound::{AsyncCallback, SubscriptionToken};

// Worker for a single host, e.g. b.siobud.com
pub async fn poll_endpoint<P: StreamStatusProvider>(
    mut rx: mpsc::UnboundedReceiver<Command>,
    polling_interval: Duration,
    status_provider: P,
    errors_until_close: Option<u32>,
) {
    let host = status_provider.get_host();
    let mut callbacks: HashMap<String, HashMap<SubscriptionToken, AsyncCallback>> = HashMap::new();
    let mut ticker = interval(polling_interval);
    let mut remaining_errors = errors_until_close.unwrap_or(3);

    'worker: loop {
        tokio::select! {
            // Process any new messages on every loop
            Some(cmd) = rx.recv() => {
                match cmd {
                    Command::AddKey(key, token, callback) => {
                        callbacks.entry(key).or_default().insert(token, callback);
                    },
                    Command::RemoveCallback(token) => {
                        callbacks.retain(|_, inner| {
                            inner.remove(&token);
                            !inner.is_empty()
                        });
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
                        // The inner contains all the different callbacks for this key
                        for (key, inner) in &callbacks {

                            // Remove it from the status list and execute all the callbacks
                            if let Some(status) = statuses.remove(key) {
                                for (_, cb) in inner {
                                    if let Err(e) = cb(status.clone()).await {
                                        println!("Callback error for key {key}: {e:?}");
                                        remaining_errors -= 1;

                                        if remaining_errors <= 0 {
                                            println!("Worker for host {host} has gone over the max error count. Closing loop.");
                                            break 'worker;
                                        }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use tokio::sync::mpsc;
    use tokio::time::Duration;
    use uuid::Uuid;

    use crate::CoreError;
    use crate::models::command::Command;
    use crate::models::status::StreamStatus;
    use crate::ports::inbound::AsyncCallback;
    use crate::ports::outbound::StreamStatusProvider;

    struct FakeProvider {
        host: String,
        statuses: HashMap<String, StreamStatus>,
    }

    #[async_trait::async_trait]
    impl StreamStatusProvider for FakeProvider {
        fn get_host(&self) -> &str {
            &self.host
        }

        async fn get_statuses(&self, keys: Vec<&str>) -> Result<HashMap<String, StreamStatus>, CoreError> {
            Ok(keys.iter().filter_map(|k| self.statuses.get(*k).map(|s| (k.to_string(), s.clone()))).collect())
        }
    }

    fn make_counting_callback(counter: Arc<Mutex<u32>>) -> AsyncCallback {
        Box::new(move |_status| {
            let counter = counter.clone();
            Box::pin(async move {
                *counter.lock().unwrap() += 1;
                Ok(())
            })
        })
    }

    #[tokio::test]
    async fn multiple_callbacks_for_same_key_are_all_called() {
        let (tx, rx) = mpsc::unbounded_channel::<Command>();
        let count_a = Arc::new(Mutex::new(0u32));
        let count_b = Arc::new(Mutex::new(0u32));

        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        tx.send(Command::AddKey("key1".to_string(), token_a, make_counting_callback(count_a.clone()))).unwrap();
        tx.send(Command::AddKey("key1".to_string(), token_b, make_counting_callback(count_b.clone()))).unwrap();

        let mut statuses = HashMap::new();
        statuses.insert("key1".to_string(), StreamStatus::Offline);

        let provider = FakeProvider { host: "test.host".to_string(), statuses };
        let handle = tokio::spawn(super::poll_endpoint(rx, Duration::from_millis(10), provider, None));

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();

        assert!(*count_a.lock().unwrap() > 0, "callback A was never called");
        assert!(*count_b.lock().unwrap() > 0, "callback B was never called");
    }

    #[tokio::test]
    async fn remove_callback_leaves_other_callbacks_intact() {
        let (tx, rx) = mpsc::unbounded_channel::<Command>();
        let count_a = Arc::new(Mutex::new(0u32));
        let count_b = Arc::new(Mutex::new(0u32));

        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        tx.send(Command::AddKey("key1".to_string(), token_a, make_counting_callback(count_a.clone()))).unwrap();
        tx.send(Command::AddKey("key1".to_string(), token_b, make_counting_callback(count_b.clone()))).unwrap();
        tx.send(Command::RemoveCallback(token_a)).unwrap();

        let mut statuses = HashMap::new();
        statuses.insert("key1".to_string(), StreamStatus::Offline);

        let provider = FakeProvider { host: "test.host".to_string(), statuses };
        let handle = tokio::spawn(super::poll_endpoint(rx, Duration::from_millis(10), provider, None));

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();

        assert_eq!(*count_a.lock().unwrap(), 0, "removed callback A should not have been called");
        assert!(*count_b.lock().unwrap() > 0, "callback B should still be called");
    }

    #[tokio::test]
    async fn worker_closes_when_last_callback_removed() {
        let (tx, rx) = mpsc::unbounded_channel::<Command>();
        let token = Uuid::new_v4();

        let count = Arc::new(Mutex::new(0u32));
        tx.send(Command::AddKey("key1".to_string(), token, make_counting_callback(count))).unwrap();
        tx.send(Command::RemoveCallback(token)).unwrap();

        let provider = FakeProvider { host: "test.host".to_string(), statuses: HashMap::new() };
        let handle = tokio::spawn(super::poll_endpoint(rx, Duration::from_millis(10), provider, None));

        let result = tokio::time::timeout(Duration::from_millis(200), handle).await;
        assert!(result.is_ok(), "worker should have exited after last callback removed");
    }
}
