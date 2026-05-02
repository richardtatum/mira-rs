use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};

use mira_core::{AsyncCallback, StreamStatus};
use serenity::all::{ChannelId, Http};

pub struct DiscordNotifier {
    key: String,
    channel_id: ChannelId,
    http: Arc<Http>,
}

impl DiscordNotifier {
    pub fn new(key: String, channel_id: ChannelId, http: Arc<Http>) -> Self {
        Self {
            key,
            channel_id,
            http,
        }
    }

    pub fn into_callback(self) -> AsyncCallback {
        let channel_id = self.channel_id;
        let http = self.http;
        let key = self.key;

        Box::new(move |status: StreamStatus| {
            let http = http.clone();
            let key = key.clone();

            Box::pin(async move {
                let message = match status {
                    StreamStatus::Online(info) => {
                        format!(
                            "{} is live! Viewers: {}, started: {}",
                            key, info.viewers, info.started
                        )
                    }
                    StreamStatus::Offline => format!("{} has gone offline.", key),
                };
                if let Err(e) = channel_id.say(&http, message).await {
                    eprintln!("Discord notification failed: {e}");
                }
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        })
    }
}
