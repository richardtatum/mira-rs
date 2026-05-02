use clap::{Parser, Subcommand};
use mira_broadcast_box::BroadcastBoxClient;
use mira_core::StreamStatusProvider;
use mira_monitor::StreamMonitor;

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
        Commands::Status {
            key,
            url,
            auth_token,
        } => {
            let client = BroadcastBoxClient::new(url, auth_token);
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
        Commands::Watch {
            key,
            url,
            auth_token,
            polling_interval,
        } => {
            let watcher = StreamMonitor::new(polling_interval);
            watcher.register_stream(url, auth_token, key.clone(), move |status| {
                let key = key.clone();
                let now = chrono::Local::now().format("%H:%M:%S");
                async move {
                    println!("[{now}] {key}: {status}");
                }
            });
            tokio::signal::ctrl_c().await.unwrap();
        }
    }
}
