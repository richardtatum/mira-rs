use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use poise::serenity_prelude::{
    self as serenity, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage, Message,
    MessageId,
};

use mira_core::{AsyncCallback, CoreError, Host, PersistenceProvider, StreamInfo, StreamStatus};
use serenity::all::{ChannelId, Http};

const EMPTY_STR: &str = "\u{200B}";

pub struct DiscordNotifier {
    host: Host,
    key: String,
    subscription_id: i64,
    http: Arc<Http>,
    persistence: Arc<dyn PersistenceProvider>,
}

impl DiscordNotifier {
    pub fn new(
        host: Host,
        key: String,
        subscription_id: i64,
        http: Arc<Http>,
        persistence: Arc<dyn PersistenceProvider>,
    ) -> Self {
        Self {
            key,
            host,
            subscription_id,
            http,
            persistence,
        }
    }

    pub fn into_callback(self) -> AsyncCallback {
        let this = Arc::new(self);

        Box::new(move |status: StreamStatus| {
            let this = this.clone();
            let subscription_id = this.subscription_id.clone();

            Box::pin(async move {
                let subscription = this
                    .persistence
                    .get_subscription(subscription_id)
                    .await
                    .unwrap();

                let message_id = subscription.message_id.map(|id| MessageId::new(id as u64));
                let channel_id = ChannelId::new(subscription.channel_id as u64);
                let playing = subscription.playing.as_deref();

                // Take the current status from the API and pair it with the presence of a message_id to determine the change
                // If a messageId is populated on a subscription, the stream is currently live. If it's null, it's offline
                match (status, message_id) {
                    (StreamStatus::Online(info), None) => {
                        this.stream_online(&channel_id, &info, playing)
                            .await
                            .unwrap();
                    }
                    (StreamStatus::Online(info), Some(message_id)) => {
                        this.stream_update(&channel_id, &message_id, &info, playing)
                            .await
                            .unwrap();
                    }
                    (StreamStatus::Offline, Some(message_id)) => {
                        this.stream_offline(&channel_id, &message_id, playing)
                            .await
                            .unwrap();
                    }
                    (StreamStatus::Offline, None) => {
                        println!("Stream {} is still offline.", this.key);
                    }
                }
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        })
    }

    async fn stream_online(
        &self,
        channel_id: &ChannelId,
        info: &StreamInfo,
        playing: Option<&str>,
    ) -> Result<(), serenity::Error> {
        let embed = self.online_embed(info, playing);

        // Send a new message and retain it's id
        let message = channel_id
            .send_message(&self.http, CreateMessage::new().embed(embed))
            .await?;

        self.mark_online(&message).await;

        Ok(())
    }

    async fn stream_update(
        &self,
        channel_id: &ChannelId,
        message_id: &MessageId,
        info: &StreamInfo,
        playing: Option<&str>,
    ) -> Result<(), serenity::Error> {
        let embed = self.online_embed(info, playing);

        channel_id
            .edit_message(&self.http, message_id, EditMessage::new().embed(embed))
            .await?;

        Ok(())
    }

    async fn stream_offline(
        &self,
        channel_id: &ChannelId,
        message_id: &MessageId,
        playing: Option<&str>,
    ) -> Result<(), serenity::Error> {
        let embed = self.offline_embed(playing);

        // Update the message in the channel
        channel_id
            .edit_message(&self.http, message_id, EditMessage::new().embed(embed))
            .await?;

        self.mark_offline().await.unwrap();

        Ok(())
    }

    // Update the database to mark the stream as online and record the message_id for future updates
    async fn mark_online(&self, message: &Message) -> Result<(), CoreError> {
        let message_id = message.id.get() as i64;
        self.persistence
            .set_subscription_message_id(self.subscription_id, message_id)
            .await
    }

    // Clear the message_id and playing values from the db to mark it as offline
    async fn mark_offline(&self) -> Result<(), CoreError> {
        self.persistence
            .clear_subscription_message_id(self.subscription_id)
            .await
    }

    fn online_embed(&self, info: &StreamInfo, playing: Option<&str>) -> CreateEmbed {
        let duration = Utc::now() - info.started;
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let duration_str = format!("{:02}:{:02}", hours, minutes);
        let viewer_str = info.viewers.to_string();
        let url = format!("{}/{}", self.host.url, self.key);

        let mut embed = CreateEmbed::new()
            .title("Stream Online")
            .url(url)
            .color(0x2ECC71)
            .description(format!("{} is streaming!", self.key))
            .field(EMPTY_STR, EMPTY_STR, false) // Add a blank line to separate the fields from the description
            .field("Duration", duration_str, true)
            .field("Viewers", viewer_str, true)
            .footer(CreateEmbedFooter::new(format!(
                "Started: {}",
                info.started.format("%d/%m/%Y, %H:%M")
            )));

        if let Some(playing) = playing {
            embed = embed.field("Playing", playing, false);
        }

        embed
    }

    fn offline_embed(&self, playing: Option<&str>) -> CreateEmbed {
        let ended = Utc::now().format("%d/%m/%Y, %H:%M").to_string();
        let url = format!("{}/{}", self.host.url, self.key);

        let mut embed = CreateEmbed::new()
            .title("Stream Offline")
            .url(url)
            .color(0xE74C3C)
            .description(format!("{} is offline.", self.key))
            .field(EMPTY_STR, EMPTY_STR, false) // Add a blank line to separate the fields from the description
            .footer(CreateEmbedFooter::new(format!("Ended: {}", ended)));

        if let Some(playing) = playing {
            embed = embed.field("Previously Playing", playing, false);
        }

        embed
    }
}
