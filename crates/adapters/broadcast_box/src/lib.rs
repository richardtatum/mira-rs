pub mod models;

use async_trait::async_trait;
use mira_core::CoreError;
use mira_core::StreamStatus;
use mira_core::StreamStatusProvider;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

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
    async fn get_status(&self, key: &str) -> Result<StreamStatus, CoreError> {
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

        // Quick dirty test for just stream online/offline
        let status = if statuses.iter().any(|s| s.stream_key == key) {
            StreamStatus::Online
        } else {
            StreamStatus::Offline
        };

        return Ok(status);
    }
}
