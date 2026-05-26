use chrono::Utc;
use mira_core::StreamInfo;
use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter};

const EMPTY_STR: &str = "\u{200B}";

pub fn error_embed(title: impl Into<String>, message: impl Into<String>) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .color(0xE74C3C)
        .description(message)
}

pub fn success_embed(title: impl Into<String>, message: impl Into<String>) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .color(0x2ECC71)
        .description(message)
}

pub fn online_embed(
    stream_url: &str,
    key: &str,
    info: &StreamInfo,
    playing: Option<&str>,
) -> CreateEmbed {
    let duration = Utc::now() - info.started;
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    let duration_str = format!("{:02}:{:02}", hours, minutes);
    let viewer_str = info.viewers.to_string();

    let mut embed = CreateEmbed::new()
        .title("Stream Online")
        .url(stream_url)
        .color(0x2ECC71)
        .description(format!("{} is streaming!", key))
        .field(EMPTY_STR, EMPTY_STR, false) // Add a blank line to separate the fields from the description
        .field("Duration", duration_str, true)
        .field("Viewers", viewer_str, true)
        .footer(CreateEmbedFooter::new(format!(
            "Started: {}",
            info.started.format("%d/%m/%Y, %H:%M")
        )));

    if let Some(playing) = playing {
        embed = embed.field("Playing", playing, false);
    }

    embed
}

pub fn offline_embed(stream_url: &str, key: &str, playing: Option<&str>) -> CreateEmbed {
    let ended = Utc::now().format("%d/%m/%Y, %H:%M").to_string();

    let mut embed = CreateEmbed::new()
        .title("Stream Offline")
        .url(stream_url)
        .color(0xE74C3C)
        .description(format!("{} is offline.", key))
        .field(EMPTY_STR, EMPTY_STR, false) // Add a blank line to separate the fields from the description
        .footer(CreateEmbedFooter::new(format!("Ended: {}", ended)));

    if let Some(playing) = playing {
        embed = embed.field("Previously Playing", playing, false);
    }

    embed
}
