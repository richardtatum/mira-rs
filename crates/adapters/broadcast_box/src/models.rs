use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VideoTrack {
    pub bitrate: i32,
    pub last_keyframe: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct AudioTrack {}

#[derive(Deserialize, Debug)]
pub struct Session {}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamSummary {
    pub stream_key: String,
    pub is_public: bool,
    pub motd: String,
    pub stream_start: DateTime<Utc>,
    pub video_tracks: Vec<VideoTrack>,
    pub sessions: Vec<Session>,
}
