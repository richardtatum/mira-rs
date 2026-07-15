//! Standalone diagnostic: performs a real WHEP offer/answer exchange, then sends a properly
//! authenticated (MESSAGE-INTEGRITY + FINGERPRINT) ICE STUN binding request straight at one of
//! BroadcastBox's returned host candidates, completely independently of str0m/is. Used to check
//! whether the lack of a response is specific to our main capture path or happens for any client.
//!
//! Usage: BROADCAST_BOX_URL=https://host STREAM_KEY=key cargo run -p mira-thumbnail --example stunclient

use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use hmac::{Hmac, Mac};
use sha1::Sha1;
use str0m::ice::{StunMessage, StunMessageBuilder, TransId};

#[tokio::main]
async fn main() {
    let url = env::var("BROADCAST_BOX_URL").expect("set BROADCAST_BOX_URL");
    let key = env::var("STREAM_KEY").expect("set STREAM_KEY");
    let auth_header = format!("Bearer {key}");
    let whep_url = format!("{url}/api/whep");

    let offer_state = mira_thumbnail::webrtc::create_offer().await.expect("create_offer failed");
    let (answer_sdp, location) =
        mira_thumbnail::whep::post_offer(&whep_url, &auth_header, &offer_state.offer_sdp).await.expect("post_offer failed");

    println!("=== answer SDP ===\n{answer_sdp}\n===================");

    let remote_ufrag = extract(&answer_sdp, "a=ice-ufrag:").expect("no remote ufrag in answer");
    let remote_pwd = extract(&answer_sdp, "a=ice-pwd:").expect("no remote pwd in answer");
    let local_ufrag = extract(&offer_state.offer_sdp, "a=ice-ufrag:").expect("no local ufrag in offer");
    let candidate_addr = first_host_candidate(&answer_sdp).expect("no host candidate in answer");

    println!("targeting {candidate_addr} with remote ufrag {remote_ufrag}");

    // Reuse the exact socket whose port was declared in our offer's candidates — BroadcastBox
    // already knows this address:port as our candidate, so this isn't relying on peer-reflexive
    // discovery for a totally unannounced address like a fresh socket would.
    let socket = offer_state.socket;

    let username = format!("{remote_ufrag}:{local_ufrag}");
    let message =
        StunMessageBuilder::new().binding().request().username(&username).prio(1).ice_controlling(1).use_candidate().build(TransId::new());

    let mut buf = [0u8; 512];
    let len = message.to_bytes(Some(remote_pwd.as_bytes()), &mut buf, hmac_sha1).expect("failed to serialize STUN message");

    let mut got_response = false;
    for attempt in 1..=5 {
        socket.send_to(&buf[..len], candidate_addr).await.expect("send failed");
        println!("attempt {attempt}: sent binding request to {candidate_addr}");

        let mut resp = [0u8; 512];
        match tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut resp)).await {
            Ok(Ok((n, from))) => {
                println!("got {n} bytes back from {from}");
                match StunMessage::parse(&resp[..n]) {
                    Ok(msg) => println!("parsed response, mapped address: {:?}", msg.mapped_address()),
                    Err(e) => println!("received a packet but failed to parse as STUN: {e}"),
                }
                got_response = true;
                break;
            }
            Ok(Err(e)) => println!("recv error: {e}"),
            Err(_) => println!("attempt {attempt}: timed out, no response"),
        }
    }

    if !got_response {
        println!("no response after 5 attempts");
    }

    mira_thumbnail::whep::delete_session(&location, &auth_header).await;
}

fn hmac_sha1(key: &[u8], parts: &[&[u8]]) -> [u8; 20] {
    let mut mac = Hmac::<Sha1>::new_from_slice(key).expect("HMAC accepts any key length");
    for part in parts {
        mac.update(part);
    }
    let mut out = [0u8; 20];
    out.copy_from_slice(&mac.finalize().into_bytes());
    out
}

fn extract<'a>(sdp: &'a str, prefix: &str) -> Option<&'a str> {
    sdp.lines().find_map(|line| line.strip_prefix(prefix))
}

fn first_host_candidate(sdp: &str) -> Option<SocketAddr> {
    sdp.lines().find_map(|line| {
        let rest = line.strip_prefix("a=candidate:")?;
        let mut parts = rest.split_whitespace();
        let (_foundation, _component, _transport, _priority) = (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
        let ip = parts.next()?;
        let port = parts.next()?;
        (parts.next()? == "typ" && parts.next()? == "host").then(|| format!("{ip}:{port}").parse().ok())?
    })
}
