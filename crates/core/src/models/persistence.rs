#[derive(Debug, Clone)]
pub struct Host {
    pub id: i64,
    pub url: String,
    pub auth_header: Option<String>,
    pub host_guild_id: i64,
}

pub enum StreamState {
    Online { message_id: i64, playing: Option<String> },
    Offline,
}

impl StreamState {
    pub fn new(message_id: Option<i64>, playing: Option<String>) -> Self {
        match message_id {
            Some(id) => StreamState::Online { message_id: id, playing },
            None => StreamState::Offline,
        }
    }
}

#[derive(Debug)]
pub struct Subscription {
    pub id: i64,
    pub key: String,
    pub host_guild_id: i64,
    pub channel_id: i64,
    pub message_id: Option<i64>,
    pub playing: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionRestore {
    pub host: Host,
    pub key: String,
    pub subscription_id: i64,
    pub channel_id: i64,
}
