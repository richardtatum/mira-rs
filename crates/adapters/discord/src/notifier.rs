use mira_core::{CoreError, StreamInfo, StreamState, StreamStatus};
use poise::serenity_prelude::{self as serenity, CreateAttachment, CreateMessage, EditMessage, MessageId};
use serenity::all::{ChannelId, Http};
use std::sync::Arc;

use crate::templates::{offline_embed, online_embed};

pub enum StateChange {
    Online { message_id: i64 },
    Offline,
}

#[derive(Clone)]
pub struct DiscordNotifier {
    http: Arc<Http>,
}

impl DiscordNotifier {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }

    pub async fn notify(
        &self,
        status: StreamStatus,
        state: StreamState,
        channel_id: ChannelId,
        stream_url: String,
        key: String,
        thumbnail: Option<Vec<u8>>,
    ) -> Result<Option<StateChange>, CoreError> {
        match (status, state) {
            (StreamStatus::Online(info), StreamState::Offline) => {
                let message = self.send_online_message(&stream_url, &key, &info, &channel_id, thumbnail).await?;
                let state_change = Some(StateChange::Online { message_id: message.get() as i64 });

                Ok(state_change)
            }
            (StreamStatus::Online(info), StreamState::Online { message_id, playing }) => {
                self.send_update_message(&stream_url, &key, &info, &channel_id, message_id, playing.as_deref(), thumbnail).await?;
                Ok(None) // No state change
            }
            (StreamStatus::Offline, StreamState::Online { message_id, playing }) => {
                self.send_offline_message(&stream_url, &key, &channel_id, message_id, playing.as_deref()).await?;
                let state_change = Some(StateChange::Offline);
                Ok(state_change)
            }
            (StreamStatus::Offline, StreamState::Offline) => {
                Ok(None) // no state change
            }
        }
    }

    async fn send_online_message(
        &self,
        stream_url: &str,
        key: &str,
        info: &StreamInfo,
        channel_id: &ChannelId,
        thumbnail: Option<Vec<u8>>,
    ) -> Result<MessageId, CoreError> {
        let has_thumb = thumbnail.is_some();
        let embed = online_embed(stream_url, key, info, None, has_thumb);

        let mut msg = CreateMessage::new().embed(embed);
        if let Some(bytes) = thumbnail {
            msg = msg.add_file(CreateAttachment::bytes(bytes, "thumb.jpg"));
        }

        // Send a new message and retain it's id
        let message = channel_id
            .send_message(&self.http, msg)
            .await
            .map_err(|e| CoreError::NotificationError(e.to_string()))?;

        return Ok(message.id);
    }

    async fn send_update_message(
        &self,
        stream_url: &str,
        key: &str,
        info: &StreamInfo,
        channel_id: &ChannelId,
        message_id: i64,
        playing: Option<&str>,
        thumbnail: Option<Vec<u8>>,
    ) -> Result<(), CoreError> {
        let has_thumb = thumbnail.is_some();
        let embed = online_embed(stream_url, key, info, playing, has_thumb);
        let message = MessageId::new(message_id as u64);

        let mut edit = EditMessage::new().embed(embed);
        if let Some(bytes) = thumbnail {
            edit = edit.new_attachment(CreateAttachment::bytes(bytes, "thumb.jpg"));
        }

        channel_id
            .edit_message(&self.http, message, edit)
            .await
            .map_err(|e| CoreError::NotificationError(e.to_string()))?;

        Ok(())
    }

    async fn send_offline_message(
        &self,
        stream_url: &str,
        key: &str,
        channel_id: &ChannelId,
        message_id: i64,
        playing: Option<&str>,
    ) -> Result<(), CoreError> {
        let embed = offline_embed(stream_url, key, playing);
        let message = MessageId::new(message_id as u64);

        // Update the message in the channel
        channel_id
            .edit_message(&self.http, message, EditMessage::new().embed(embed))
            .await
            .map_err(|e| CoreError::NotificationError(e.to_string()))?;

        Ok(())
    }
}
