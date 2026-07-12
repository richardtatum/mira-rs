pub mod whep;
pub mod webrtc;
pub mod codec;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("WHEP signaling failed: {0}")]
    Signaling(String),
    #[error("timed out waiting for keyframe")]
    Timeout,
    #[error("decode or encode failed: {0}")]
    Decode(String),
}

pub async fn capture(whep_url: &str, auth_header: &str) -> Result<Vec<u8>, ThumbnailError> {
    let offer_state = webrtc::create_offer().await?;
    let offer_sdp = offer_state.offer_sdp.clone();

    let (sdp_answer, location) = whep::post_offer(whep_url, auth_header, &offer_sdp).await?;

    // Extract the keyframe but teardown regardless of the result
    let keyframe_result = webrtc::extract_keyframe(offer_state, &sdp_answer).await;
    whep::delete_session(&location, auth_header).await;
    let keyframe = keyframe_result?;

    codec::decode_and_encode(&keyframe)
}
