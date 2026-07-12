use std::net::SocketAddr;
use std::time::{Duration, Instant};
use str0m::change::{SdpAnswer, SdpPendingOffer};
use str0m::media::{Direction, MediaKind};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, Input, Output, Rtc};
use tokio::net::UdpSocket;

const STUN_SERVER: &str = "stun.cloudflare.com:3478";

pub struct OfferState {
    pub offer_sdp: String,
    pub rtc: Rtc,
    pub pending: SdpPendingOffer,
    pub socket: UdpSocket,
    pub local_addr: SocketAddr,
}

pub async fn create_offer() -> Result<OfferState, crate::ThumbnailError> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| crate::ThumbnailError::Signaling(format!("socket bind failed: {e}")))?;
    let local_addr = socket.local_addr().map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;

    let mut rtc = Rtc::new(Instant::now());

    // Add the local host candidate so ICE has something to work with even if STUN fails
    if let Ok(host) = Candidate::host(local_addr, Protocol::Udp) {
        rtc.add_local_candidate(host);
    }

    // Discover our public address via STUN and add a server-reflexive candidate.
    // Required when the bot is behind NAT — BroadcastBox runs full ICE (not ICE lite)
    // and will initiate connectivity checks back to us using the candidates in the SDP offer.
    if let Some(public_addr) = stun_gather(&socket, STUN_SERVER).await {
        if let Ok(srflx) = Candidate::server_reflexive(public_addr, local_addr, Protocol::Udp) {
            rtc.add_local_candidate(srflx);
        }
    }

    let mut change = rtc.sdp_api();
    change.add_media(MediaKind::Video, Direction::RecvOnly, None, None, None);

    // ponytail: apply() returns Option — None means no changes, which can't happen here
    let (offer, pending) =
        change.apply().ok_or_else(|| crate::ThumbnailError::Signaling("SDP apply returned no offer".into()))?;

    Ok(OfferState { offer_sdp: offer.to_sdp_string(), rtc, pending, socket, local_addr })
}

/// Send a STUN binding request and return our public SocketAddr from XOR-MAPPED-ADDRESS.
/// Returns None on any error or timeout — caller falls back to host-only candidates.
async fn stun_gather(socket: &UdpSocket, server: &str) -> Option<SocketAddr> {
    const MAGIC: u32 = 0x2112_A442;

    // Derive a stable transaction ID from our local port (avoids a rand dep)
    let port = socket.local_addr().ok()?.port();
    let mut txid = [0u8; 12];
    txid[..2].copy_from_slice(&port.to_be_bytes());

    // Minimal 20-byte STUN binding request — no attributes needed
    let mut req = [0u8; 20];
    req[0..2].copy_from_slice(&0x0001u16.to_be_bytes()); // Binding Request
    req[2..4].copy_from_slice(&0u16.to_be_bytes()); // message length = 0
    req[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    req[8..20].copy_from_slice(&txid);

    let stun_addr = tokio::net::lookup_host(server).await.ok()?.next()?;
    socket.send_to(&req, stun_addr).await.ok()?;

    // Wait up to 3 seconds for the response
    let mut buf = [0u8; 256];
    let (n, _) = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf)).await.ok()?.ok()?;

    let resp = &buf[..n];
    if resp.len() < 20 {
        return None;
    }

    // Validate magic cookie and transaction ID
    if resp[4..8] != MAGIC.to_be_bytes() {
        return None;
    }
    if resp[8..20] != txid {
        return None;
    }

    // Walk attributes looking for XOR-MAPPED-ADDRESS (type 0x0020)
    let attr_section_len = u16::from_be_bytes([resp[2], resp[3]]) as usize;
    let attrs = resp.get(20..20 + attr_section_len)?;
    let mut i = 0;
    while i + 4 <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[i], attrs[i + 1]]);
        let attr_val_len = u16::from_be_bytes([attrs[i + 2], attrs[i + 3]]) as usize;

        if attr_type == 0x0020 && attr_val_len >= 8 && attrs.get(i + 5)? == &0x01 {
            // IPv4: port XOR'd with top 16 bits of magic, IP XOR'd with full magic
            let port = u16::from_be_bytes([attrs[i + 6], attrs[i + 7]]) ^ (MAGIC >> 16) as u16;
            let ip = [attrs[i + 8] ^ 0x21, attrs[i + 9] ^ 0x12, attrs[i + 10] ^ 0xA4, attrs[i + 11] ^ 0x42];
            return Some(SocketAddr::from((ip, port)));
        }

        // Attributes are padded to 4-byte boundaries
        i += 4 + attr_val_len;
        if attr_val_len % 4 != 0 {
            i += 4 - (attr_val_len % 4);
        }
    }

    None
}

pub async fn extract_keyframe(state: OfferState, sdp_answer: &str) -> Result<Vec<u8>, crate::ThumbnailError> {
    tokio::time::timeout(Duration::from_secs(10), do_extract(state, sdp_answer))
        .await
        .unwrap_or(Err(crate::ThumbnailError::Timeout))
}

async fn do_extract(state: OfferState, sdp_answer: &str) -> Result<Vec<u8>, crate::ThumbnailError> {
    let OfferState { mut rtc, pending, socket, local_addr, .. } = state;

    let answer = SdpAnswer::from_sdp_string(sdp_answer)
        .map_err(|e| crate::ThumbnailError::Signaling(format!("invalid SDP answer: {e}")))?;

    // accept_answer takes (self, pending, answer) and mutates rtc via the SdpApi borrow
    rtc.sdp_api()
        .accept_answer(pending, answer)
        .map_err(|e| crate::ThumbnailError::Signaling(format!("accept_answer failed: {e}")))?;

    let mut buf = vec![0u8; 2048];

    loop {
        let output = rtc.poll_output().map_err(|e| crate::ThumbnailError::Signaling(format!("poll_output: {e}")))?;

        match output {
            Output::Transmit(send) => {
                println!("Is output transmit");
                socket
                    .send_to(&send.contents, send.destination)
                    .await
                    .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;
            }
            Output::Timeout(deadline) => {
                let delay = deadline.saturating_duration_since(Instant::now());
                tokio::select! {
                    result = socket.recv_from(&mut buf) => {
                        let (n, source) = result
                            .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;
                        println!("recv {n} bytes from {source}");
                        let receive = Receive::new(Protocol::Udp, source, local_addr, &buf[..n])
                            .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;
                        rtc.handle_input(Input::Receive(Instant::now(), receive))
                            .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;
                    }
                    _ = tokio::time::sleep(delay) => {
                        println!("Select timed out, no packet");
                        rtc.handle_input(Input::Timeout(Instant::now()))
                            .map_err(|e| crate::ThumbnailError::Signaling(e.to_string()))?;
                    }
                }
            }
            Output::Event(Event::MediaData(data)) => {
                println!("Is output event");
                if is_keyframe(&data.data) {
                    println!("Is keyframe");
                    return Ok(data.data.to_vec());
                }
            }
            Output::Event(_) => {}
        }
    }
}

fn is_keyframe(data: &[u8]) -> bool {
    let mut i = 0;
    while i + 4 <= data.len() {
        let start = if data[i..].starts_with(&[0, 0, 0, 1]) {
            Some(i + 4)
        } else if data[i..].starts_with(&[0, 0, 1]) {
            Some(i + 3)
        } else {
            None
        };
        if let Some(nal_start) = start {
            if nal_start < data.len() && (data[nal_start] & 0x1F) == 5 {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_idr_nal_unit_in_annex_b() {
        // Annex B start code + NAL type 5 (IDR slice) header byte
        let frame = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x00];
        assert!(is_keyframe(&frame));
    }

    #[test]
    fn rejects_non_idr_nal_unit() {
        // NAL type 1 (non-IDR slice)
        let frame = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x00];
        assert!(!is_keyframe(&frame));
    }
}
