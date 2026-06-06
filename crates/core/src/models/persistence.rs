use uuid::Uuid;

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

#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: i64,
    pub key: String,
    pub channel_id: i64,
    pub token: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct HostSubscription {
    pub host: Host,
    pub subscription: Subscription,
}

impl HostSubscription {
    pub fn get_url(&self) -> String {
        format!("{}/{}", self.host.url, self.subscription.key)
    }
}
