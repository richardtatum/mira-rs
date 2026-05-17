use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use poise::serenity_prelude::{
    self as serenity, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage, MessageId,
};

use mira_core::{AsyncCallback, PersistenceProvider, StreamInfo, StreamStatus};
use serenity::all::{ChannelId, Http};

pub struct DiscordNotifier {
    key: String,
    host_guild_id: i64,
    channel_id: ChannelId,
    http: Arc<Http>,
    persistence: Arc<dyn PersistenceProvider>,
}

impl DiscordNotifier {
    pub fn new(
        key: String,
        host_guild_id: i64,
        channel_id: ChannelId,
        http: Arc<Http>,
        persistence: Arc<dyn PersistenceProvider>,
    ) -> Self {
        Self {
            key,
            host_guild_id,
            channel_id,
            http,
            persistence,
        }
    }

    pub fn into_callback(self) -> AsyncCallback {
        let this = Arc::new(self);

        Box::new(move |status: StreamStatus| {
            let this = this.clone();

            Box::pin(async move {
                let channel_id_i64 = this.channel_id.get() as i64;
                let subscription = this
                    .persistence
                    .get_subscription(this.key.clone(), this.host_guild_id, channel_id_i64)
                    .await
                    .unwrap();

                let playing = subscription.playing.as_deref();

                match (status, subscription.message_id) {
                    (StreamStatus::Online(info), Some(message_id)) => {
                        this.stream_update(message_id, &info, playing)
                            .await
                            .unwrap();
                    }
                    (StreamStatus::Online(info), None) => {
                        this.stream_online(&info, playing, subscription.id)
                            .await
                            .unwrap();
                    }
                    (StreamStatus::Offline, Some(message_id)) => {
                        this.stream_offline(message_id, playing, subscription.id)
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
        info: &StreamInfo,
        playing: Option<&str>,
        subscription_id: i64,
    ) -> Result<(), serenity::Error> {
        let embed = self.online_embed(info, playing);
        let message = self
            .channel_id
            .send_message(&self.http, CreateMessage::new().embed(embed))
            .await?;
        self.persistence
            .set_subscription_message(subscription_id, message.id.get() as i64)
            .await
            .unwrap();
        Ok(())
    }

    async fn stream_update(
        &self,
        message_id: i64,
        info: &StreamInfo,
        playing: Option<&str>,
    ) -> Result<(), serenity::Error> {
        let embed = self.online_embed(info, playing);
        let message = MessageId::new(message_id as u64);
        self.channel_id
            .edit_message(&self.http, message, EditMessage::new().embed(embed))
            .await?;
        Ok(())
    }

    async fn stream_offline(
        &self,
        message_id: i64,
        playing: Option<&str>,
        subscription_id: i64,
    ) -> Result<(), serenity::Error> {
        let embed = self.offline_embed(playing);
        let message = MessageId::new(message_id as u64);
        self.channel_id
            .edit_message(&self.http, message, EditMessage::new().embed(embed))
            .await?;
        self.persistence
            .clear_subscription_message(subscription_id)
            .await
            .unwrap();
        Ok(())
    }

    fn online_embed(&self, info: &StreamInfo, playing: Option<&str>) -> CreateEmbed {
        let duration = Utc::now() - info.started;
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let duration_str = format!("{:02}:{:02}", hours, minutes);

        let mut embed = CreateEmbed::new()
            .title("Stream Online")
            .color(0x2ECC71)
            .description(format!("{} is streaming!", self.key))
            .field("Duration", duration_str, true)
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
        let mut embed = CreateEmbed::new()
            .title("Stream Offline")
            .color(0xE74C3C)
            .description(format!("{} is offline.", self.key))
            .footer(CreateEmbedFooter::new(format!("Ended: {}", ended)));

        if let Some(playing) = playing {
            embed = embed.field("Previously Playing", playing, false);
        }

        embed
    }
}
