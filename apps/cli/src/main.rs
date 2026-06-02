use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};
use mira_broadcast_box::BroadcastBoxClient;
use mira_core::{StreamStatus, StreamStatusProvider};
use mira_stream_watcher::StreamWatcher;

#[derive(Parser)]
#[command(name = "mira", about = "MIRA CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check whether a stream key is online or offline
    Status {
        /// The stream key to check
        key: String,
        /// BroadcastBox base URL
        #[arg(long, env = "BROADCAST_BOX_URL")]
        url: String,
        /// Optional bearer token for authentication
        #[arg(long, env = "BROADCAST_BOX_AUTH_TOKEN")]
        auth_token: Option<String>,
    },
    Watch {
        /// The stream key to check
        key: String,
        /// BroadcastBox base URL
        #[arg(long, env = "BROADCAST_BOX_URL")]
        url: String,
        /// Optional bearer token for authentication
        #[arg(long, env = "BROADCAST_BOX_AUTH_TOKEN")]
        auth_token: Option<String>,
        /// Optional host polling interval in seconds
        #[arg(long)]
        polling_interval: Option<u64>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status { key, url, auth_token } => {
            let client = BroadcastBoxClient::new(url, auth_token).expect("Failed to create broadcast box client!");
            match client.get_statuses(vec![&key]).await {
                Ok(status) => {
                    if let Some(stream_status) = status.get(&key) {
                        println!("{key}: {stream_status}")
                    } else {
                        eprintln!("Stream not found! Exiting.");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Watch { key, url, auth_token, polling_interval } => {
            let watcher = StreamWatcher::new(polling_interval);
            let curr_status: Arc<Mutex<Option<StreamStatus>>> = Arc::new(Mutex::new(None));

            watcher
                .watch(url, auth_token, key.clone(), move |status| {
                    let key = key.clone();
                    let curr_status = curr_status.clone();

                    async move {
                        let (changed, has_prev) = {
                            let prev_status = curr_status.lock().unwrap();

                            let changed = match (&*prev_status, &status) {
                                (None, _) => true,
                                (Some(StreamStatus::Online(_)), StreamStatus::Online(_)) => true,
                                (Some(StreamStatus::Online(_)), StreamStatus::Offline) => true,
                                (Some(StreamStatus::Offline), StreamStatus::Online(_)) => true,
                                _ => false,
                            };

                            (changed, prev_status.is_some())
                        };

                        if changed {
                            let now = chrono::Local::now().format("%H:%M:%S");

                            // If we have printed a status previously, move up a line
                            if has_prev {
                                print!("\x1B[1A\x1B[2K");
                            }

                            println!("[{now}] {key}: {status}");
                            *curr_status.lock().unwrap() = Some(status); // Set the latest value of the status
                        }

                        Ok(())
                    }
                })
                .expect("Watcher failed!");

            tokio::signal::ctrl_c().await.unwrap();
        }
    }
}
