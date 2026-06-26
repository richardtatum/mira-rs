use core::time;
use std::collections::HashMap;

use mira_core::{Host, PersistenceProvider};

use crate::{
    Context, Error,
    templates::{error_embed, success_embed},
};
use poise::{
    CreateReply,
    futures_util::StreamExt as _,
    serenity_prelude::{
        ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption,
    },
};

#[poise::command(slash_command)]
pub async fn subscribe<P: PersistenceProvider>(
    ctx: Context<'_, P>,
    #[description = "Which key to subscribe to"] key: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Subscribe Failed", "/subscribe can only be ran from a server channel.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    };

    let user_id = ctx.author().id;
    let channel_id = ctx.channel_id();

    let hosts = ctx.data().subscription_handler.get_hosts(guild_id).await?;
    if hosts.is_empty() {
        let embed = error_embed(
            "No Hosts",
            "There are no hosts configured for this server. Please add a host first with /host",
        );
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    }

    let hosts_by_id: HashMap<i64, Host> = hosts.into_iter().map(|h| (h.id, h)).collect();

    // Build the select host component
    let options = hosts_by_id.values().map(|h| CreateSelectMenuOption::new(&h.url, h.id.to_string())).collect();
    let reply = CreateReply::default().ephemeral(true).content("Select a host to remove:").components(vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new("host-select", CreateSelectMenuKind::String { options }).placeholder("Choose a host"),
        ),
    ]);

    let sent = ctx.send(reply).await?;
    let message_id = sent.message().await?.id;

    // Wait for host selection
    let mut select_stream = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.message.id == message_id)
        .stream();

    // Handle timeout
    let Some(select_interaction) = select_stream.next().await else {
        let embed = error_embed("Timed Out", "No response received.");
        sent.edit(ctx, CreateReply::default().content("").embed(embed).components(vec![])).await?;
        return Ok(());
    };

    // Action the subscribe
    let ComponentInteractionDataKind::StringSelect { values } = &select_interaction.data.kind else {
        return Ok(());
    };

    let Some(host_id): Option<i64> = values.first().and_then(|v| v.parse().ok()) else {
        return Ok(());
    };

    let Some(host) = hosts_by_id.get(&host_id).cloned() else {
        let embed = error_embed("Error", "The selected host no longer exists. Please try again.");
        let response = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new().content("").embed(embed).components(vec![]),
        );
        select_interaction.create_response(ctx.serenity_context(), response).await?;
        return Ok(());
    };

    let host_url = host.url.clone();

    ctx.data().subscription_handler.subscribe(host, key.clone(), user_id, channel_id).await?;

    // Replace the ephemeral select menu with a success message
    let ack = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .content("")
            .embed(success_embed("Done", "Subscription added."))
            .components(vec![]),
    );
    select_interaction.create_response(&ctx.serenity_context(), ack).await?;

    // Send a public success message to the channel
    let embed = success_embed(
        "Success",
        format!("Subscribed to {}/{}. You will be notified in this channel when they are next online.", host_url, key),
    );
    channel_id.send_message(&ctx.serenity_context(), CreateMessage::new().embed(embed)).await?;

    Ok(())
}
