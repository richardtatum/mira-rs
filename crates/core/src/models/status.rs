pub struct StreamInfo {
    pub started: String,
    pub viewers: u32,
}

pub enum StreamStatus {
    Online(StreamInfo),
    Offline,
}

impl StreamStatus {
    pub fn to_db_string(&self) -> &str {
        match self {
            StreamStatus::Online(_) => "online",
            StreamStatus::Offline => "offline",
        }
    }

    pub fn from_db(status: &str, started: Option<String>, viewers: Option<u32>) -> Self {
        match status {
            "online" => StreamStatus::Online(StreamInfo {
                started: started.unwrap_or_default(),
                viewers: viewers.unwrap_or(0),
            }),
            _ => StreamStatus::Offline,
        }
    }
}

impl std::fmt::Display for StreamStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamStatus::Online(info) => write!(
                f,
                "Online. Started: {}, viewers: {}",
                info.started, info.viewers
            ),
            StreamStatus::Offline => write!(f, "Offline"),
        }
    }
}
