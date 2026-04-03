use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VideoTrack {
    pub bitrate: i32,
    pub last_keyframe: String, // Datetime
}

#[derive(Deserialize, Debug)]
pub struct AudioTrack {}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamSummary {
    pub stream_key: String,
    pub is_public: bool,
    pub motd: String,
    pub stream_start: String, // Datetime
    pub video_tracks: Vec<VideoTrack>,
}
