use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, EditMessage, MessageId};

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

                match (status, subscription.message_id) {
                    (StreamStatus::Online(info), Some(message_id)) => {
                        this.stream_update(message_id, &info).await.unwrap();
                    }
                    (StreamStatus::Online(info), None) => {
                        this.stream_online(&info, subscription.id).await.unwrap();
                    }
                    (StreamStatus::Offline, Some(message_id)) => {
                        this.stream_offline(message_id, subscription.id)
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
        subscription_id: i64,
    ) -> Result<(), serenity::Error> {
        let text = format!(
            "{} is now live! Viewers: {}, started: {}",
            self.key, info.viewers, info.started
        );
        let message = self.channel_id.say(&self.http, text).await?;
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
    ) -> Result<(), serenity::Error> {
        let text = format!(
            "Stream {} is still online. Viewers: {}",
            self.key, info.viewers
        );
        let message = MessageId::new(message_id as u64);
        self.channel_id
            .edit_message(&self.http, message, EditMessage::new().content(text))
            .await?;
        Ok(())
    }

    async fn stream_offline(
        &self,
        message_id: i64,
        subscription_id: i64,
    ) -> Result<(), serenity::Error> {
        let text = format!("Stream {} is now offline.", self.key);
        let message = MessageId::new(message_id as u64);
        self.channel_id
            .edit_message(&self.http, message, EditMessage::new().content(text))
            .await?;
        self.persistence
            .clear_subscription_message(subscription_id)
            .await
            .unwrap();
        Ok(())
    }
}
