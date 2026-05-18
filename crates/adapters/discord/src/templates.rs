use poise::serenity_prelude::CreateEmbed;

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
