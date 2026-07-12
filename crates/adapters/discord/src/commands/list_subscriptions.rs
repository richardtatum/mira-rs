use std::collections::HashMap;

use mira_core::PersistenceProvider;
use poise::CreateReply;
use poise::serenity_prelude::CreateEmbed;

use crate::{Context, Error, templates::error_embed};

#[poise::command(slash_command)]
pub async fn list_subscriptions<P: PersistenceProvider>(ctx: Context<'_, P>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("List Failed", "/list_subscriptions can only be ran from a server channel currently.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    };

    let subscriptions = ctx.data().subscription_handler.get_subscriptions(guild_id).await?;
    if subscriptions.is_empty() {
        let embed = error_embed("No Subscriptions", "There are no active subscriptions for this server.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    }

    let mut subscriptions_by_host: HashMap<String, Vec<String>> = HashMap::new();
    for sub in &subscriptions {
        // Get the entry for this host, creating a new mutable array if it's empty, then push the subscription key
        subscriptions_by_host.entry(sub.host.url.clone()).or_default().push(sub.subscription.key.clone());
    }

    let mut embed = CreateEmbed::new().title("Subscriptions").color(0x3498DB);
    for (host_url, keys) in &subscriptions_by_host {
        // Create an embed field for each host, with the url as the header and keys as a list within
        let subsciption_keys = keys.iter().map(|k| format!("• {k}")).collect::<Vec<_>>().join("\n");
        embed = embed.field(host_url, subsciption_keys, false);
    }

    ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;

    Ok(())
}
