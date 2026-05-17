#[derive(Debug)]
pub struct Host {
    pub id: i64,
    pub url: String,
    pub auth_header: Option<String>,
    pub host_guild_id: i64,
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

impl Subscription {
    pub fn is_online(&self) -> bool {
        self.message_id.is_some()
    }
}
