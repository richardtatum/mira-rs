use chrono::{DateTime, Utc};

pub struct StreamInfo {
    pub started: DateTime<Utc>,
    pub viewers: u32,
}

pub enum StreamStatus {
    Online(StreamInfo),
    Offline,
}

impl std::fmt::Display for StreamStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamStatus::Online(info) => write!(
                f,
                "Online. Started: {}, viewers: {}",
                info.started.format("%d/%m/%Y, %H:%M"),
                info.viewers
            ),
            StreamStatus::Offline => write!(f, "Offline"),
        }
    }
}
