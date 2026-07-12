use reqwest::Client;

pub async fn post_offer(
    url: &str,
    auth_header: &str,
    sdp_offer: &str,
) -> Result<(String, String), crate::ThumbnailError> {
    let client = Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/sdp")
        .header("Authorization", auth_header)
        .body(sdp_offer.to_string())
        .send()
        .await
        .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;

    if response.status().as_u16() != 201 {
        return Err(crate::ThumbnailError::Signaling(
            format!("expected 201, got {}", response.status()),
        ));
    }

    let location = response
        .headers()
        .get("Location")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| crate::ThumbnailError::Signaling("missing Location header".into()))?
        .to_string();

    let answer = response
        .text()
        .await
        .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;

    Ok((answer, location))
}

pub async fn delete_session(location_url: &str, auth_header: &str) {
    let _ = Client::new()
        .delete(location_url)
        .header("Authorization", auth_header)
        .send()
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn post_offer_returns_signaling_error_on_non_201() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/whep"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let result = post_offer(
            &format!("{}/api/whep", server.uri()),
            "Bearer test-token",
            "v=0\r\n",
        ).await;

        assert!(matches!(result, Err(crate::ThumbnailError::Signaling(_))));
    }

    #[tokio::test]
    async fn post_offer_returns_answer_and_location_on_201() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/whep"))
            .respond_with(
                ResponseTemplate::new(201)
                    .append_header("Location", "/api/whep/session-abc")
                    .set_body_string("v=0\r\na=recvonly\r\n"),
            )
            .mount(&server)
            .await;

        let (answer, location) = post_offer(
            &format!("{}/api/whep", server.uri()),
            "Bearer test-token",
            "v=0\r\n",
        ).await.unwrap();

        assert!(answer.contains("v=0"));
        assert!(location.contains("session-abc"));
    }
}
