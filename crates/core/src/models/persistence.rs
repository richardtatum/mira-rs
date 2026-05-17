use crate::StreamStatus;

pub struct Host {
    pub id: u64,
    pub url: String,
    pub auth_header: String,
    pub guild_id: u64,
    pub created_by: u64,
}

pub struct Subscription {
    pub id: u64,
    pub guild_id: u64,
    pub host_id: u64,
    pub key: String,
    pub channel_id: u64,
    pub created_by: u64,
}

pub struct Stream {
    pub id: u64,
    pub subscription_id: u64,
    pub status: StreamStatus,
    pub viewer_count: u64,
    pub message_id: u64, // The message to edit with updates
    pub playing: String,
    pub start_time: String,
    pub end_time: String,
}
