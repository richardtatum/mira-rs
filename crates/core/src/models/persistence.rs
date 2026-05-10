use crate::StreamStatus;

pub struct Host {
    pub id: u64,
    pub url: String,
    pub auth_header: String,
    pub guild_id: u64,
    pub created_by: u64,
}

pub struct Subscription {
    id: u64,
    host_id: u64,
    key: String,
    channel_id: u64,
    created_by: u64,
}

pub struct Stream {
    id: u64,
    subscription_id: u64,
    status: StreamStatus,
    viewer_count: u64,
    message_id: u64, // The message to edit with updates
    playing: String,
    start_time: String,
    end_time: String,
}
