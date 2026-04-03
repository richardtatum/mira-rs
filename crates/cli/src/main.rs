use broadcast_box::BroadcastBoxClient;
use clap::{Parser, Subcommand};
use mira_core::{StreamStatus, StreamStatusProvider};

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
        #[arg(long, env = "BROADCAST_BOX_TOKEN")]
        token: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status { key, url, token } => {
            let client = BroadcastBoxClient::new(url, token);
            match client.get_status(&key).await {
                Ok(StreamStatus::Online) => println!("{key}: online"),
                Ok(StreamStatus::Offline) => println!("{key}: offline"),
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
