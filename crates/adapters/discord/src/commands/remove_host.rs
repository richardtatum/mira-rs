use core::time;
use std::collections::HashMap;

use mira_core::{Host, PersistenceProvider};
use poise::{
    CreateReply,
    futures_util::StreamExt as _,
    serenity_prelude::{
        ButtonStyle, ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateButton,
        CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu,
        CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse,
    },
};

use crate::{
    Context, Error,
    templates::{error_embed, success_embed},
};

#[poise::command(slash_command)]
pub async fn remove_host<P: PersistenceProvider>(ctx: Context<'_, P>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Error", "/remove_host can only be run from a server channel.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    };

    let hosts = ctx.data().subscription_handler.get_hosts(guild_id).await?;
    if hosts.is_empty() {
        let embed = error_embed("No Hosts", "There are no hosts configured for this server.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    }

    let hosts_by_id: HashMap<i64, Host> = hosts.into_iter().map(|h| (h.id, h)).collect();

    // Build the select host component
    let options: Vec<CreateSelectMenuOption> =
        hosts_by_id.values().map(|h| CreateSelectMenuOption::new(&h.url, h.id.to_string())).collect();

    let reply = CreateReply::default().ephemeral(true).content("Select a host to remove:").components(vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new("host-select", CreateSelectMenuKind::String { options }).placeholder("Choose a host"),
        ),
    ]);

    let sent = ctx.send(reply).await?;
    let message_id = sent.message().await?.id;

    // Stage 1: wait for host selection
    let mut select_stream = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.message.id == message_id)
        .stream();

    let Some(select_interaction) = select_stream.next().await else {
        let embed = error_embed("Timed Out", "No response received.");
        sent.edit(ctx, CreateReply::default().content("").embed(embed).components(vec![])).await?;
        return Ok(());
    };

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

    // Gather subscriptions under this host to show in confirmation
    let all_subs = ctx.data().subscription_handler.get_subscriptions(guild_id).await?;
    let host_keys: Vec<String> = all_subs
        .iter()
        .filter(|hs| hs.host.host_guild_id == host.host_guild_id)
        .map(|hs| hs.subscription.key.clone())
        .collect();

    let keys_display = if host_keys.is_empty() {
        "*(no active subscriptions)*".to_string()
    } else {
        host_keys.iter().map(|k| format!("• {}", k)).collect::<Vec<_>>().join("\n")
    };

    // Show confirmation
    let confirm_embed =
        error_embed("Confirm Removal", format!("**Host:** {}\n\n**Subscribed keys:**\n{}", host.url, keys_display));

    let confirm_response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().content("").embed(confirm_embed).components(vec![
            CreateActionRow::Buttons(vec![
                CreateButton::new("confirm").label("Remove").style(ButtonStyle::Danger),
                CreateButton::new("cancel").label("Cancel").style(ButtonStyle::Secondary),
            ]),
        ]),
    );
    select_interaction.create_response(ctx.serenity_context(), confirm_response).await?;

    // Wait for button click
    let mut button_stream = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.message.id == message_id)
        .stream();

    // If it times out, update the select component with a timeout message
    let Some(button_interaction) = button_stream.next().await else {
        select_interaction
            .edit_response(
                ctx.serenity_context(),
                EditInteractionResponse::new()
                    .content("")
                    .embed(error_embed("Timed Out", "No response received."))
                    .components(vec![]),
            )
            .await
            .ok();
        return Ok(());
    };

    if button_interaction.data.custom_id == "confirm" {
        ctx.data().subscription_handler.remove_host(host.clone(), guild_id).await?;

        // Update delete confirmation to inform the user that requested the change
        let done_response = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(success_embed("Done", "Host removed."))
                .content("")
                .components(vec![]),
        );

        button_interaction.create_response(ctx.serenity_context(), done_response).await?;

        // Inform everyone in the channel of the host/keys that have been unsubscribed
        let keys_display_public = if host_keys.is_empty() {
            "*(none)*".to_string()
        } else {
            host_keys.iter().map(|k| format!("• {}", k)).collect::<Vec<_>>().join("\n")
        };

        let public_embed = success_embed(
            "Host Removed",
            format!("**{}** has been removed.\n\n**Unsubscribed keys:**\n{}", host.url, keys_display_public),
        );

        ctx.channel_id().send_message(ctx.serenity_context(), CreateMessage::new().embed(public_embed)).await?;
    } else {
        let cancel_response = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .embed(error_embed("Cancelled", "No changes were made."))
                .content("")
                .components(vec![]),
        );
        button_interaction.create_response(ctx.serenity_context(), cancel_response).await?;
    }

    Ok(())
}
