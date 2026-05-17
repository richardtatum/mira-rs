use crate::StreamStatus;

#[derive(Debug)]
pub struct Host {
    pub id: i64,
    pub url: String,
    pub auth_header: Option<String>,
    pub guild_id: i64,
    pub created_by: i64,
}

pub struct Subscription {
    pub id: i64,
    pub guild_id: i64,
    pub host_id: i64,
    pub key: String,
    pub channel_id: i64,
    pub created_by: i64,
}

pub struct Stream {
    pub id: i64,
    pub subscription_id: i64,
    pub status: StreamStatus,
    pub viewer_count: i64,
    pub message_id: i64, // The message to edit with updates
    pub playing: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
}
