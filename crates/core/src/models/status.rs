pub struct StreamInfo {
    pub started: String,
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
                info.started, info.viewers
            ),
            StreamStatus::Offline => write!(f, "Offline"),
        }
    }
}
