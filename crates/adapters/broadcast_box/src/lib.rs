pub mod models;

use std::collections::HashMap;

use async_trait::async_trait;
use mira_core::{CoreError, StreamInfo, StreamStatus, StreamStatusProvider};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::models::StreamSummary;

pub struct BroadcastBoxClient {
    base_url: String,
    client: reqwest::Client,
}

impl BroadcastBoxClient {
    pub fn new(base_url: String, bearer_token: Option<String>) -> Self {
        let mut headers = HeaderMap::new();
        if let Some(bearer) = bearer_token {
            let header_value = HeaderValue::from_str(&bearer).unwrap();
            headers.insert(AUTHORIZATION, header_value);
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self { base_url, client }
    }
}

#[async_trait]
impl StreamStatusProvider for BroadcastBoxClient {
    fn get_host(&self) -> &str {
        &self.base_url
    }

    async fn get_statuses(
        &self,
        keys: Vec<&str>,
    ) -> Result<HashMap<String, StreamStatus>, CoreError> {
        let url = format!("{0}/api/status", self.base_url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        let statuses = response
            .json::<Vec<models::StreamSummary>>()
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?; // Maybe should be a parse error?

        let status_by_key: HashMap<String, StreamSummary> = statuses
            .into_iter()
            .map(|status| (status.stream_key.clone(), status))
            .collect();

        let result: HashMap<String, StreamStatus> = keys
            .into_iter()
            .map(|key| {
                let status = if let Some(online) = status_by_key.get(key) {
                    let started = online.stream_start.format("%Y-%m-%d %H:%M:%S").to_string();
                    let viewers = online.sessions.iter().count() as u32;

                    StreamStatus::Online(StreamInfo { started, viewers })
                } else {
                    StreamStatus::Offline
                };

                (key.into(), status)
            })
            .collect();

        return Ok(result);
    }
}
