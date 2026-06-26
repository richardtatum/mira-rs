use core::time;
use std::collections::HashMap;

use mira_core::{HostSubscription, PersistenceProvider};
use poise::{
    CreateReply,
    futures_util::StreamExt as _,
    serenity_prelude::{
        ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
};

use crate::{
    Context, Error,
    templates::{error_embed, success_embed},
};

#[poise::command(slash_command)]
pub async fn unsubscribe<P: PersistenceProvider>(ctx: Context<'_, P>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Subscribe Failed", "/unsubscribe can only be ran from a server channel.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    };

    // Obtain all the subscriptions for the guild
    let subscriptions = ctx.data().subscription_handler.get_subscriptions(guild_id).await?;
    if subscriptions.is_empty() {
        let embed = error_embed("No Subscriptions", "There are no subscriptions to unsubscribe from.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    }

    let subscriptions_by_id: HashMap<i64, HostSubscription> =
        subscriptions.into_iter().map(|s| (s.subscription.id, s)).collect();

    let options = subscriptions_by_id
        .iter()
        .map(|(id, sub)| CreateSelectMenuOption::new(&sub.get_url(), &id.to_string()))
        .collect();

    // Create a select menu
    let reply = CreateReply::default().ephemeral(true).content("Choose a subscription").components(vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new("subscription-select", CreateSelectMenuKind::String { options })
                .placeholder("Choose a subscription"),
        ),
    ]);

    let sent = ctx.send(reply).await?;
    let message_id = sent.message().await?.id;

    // Wait for the selection
    let mut stream = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.message.id == message_id)
        .stream();

    let Some(interaction) = stream.next().await else {
        let embed = error_embed("Timed Out", "No response received.");
        sent.edit(ctx, CreateReply::default().content("").embed(embed).components(vec![])).await.ok();
        return Ok(());
    };

    let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind else {
        return Ok(());
    };

    let Some(subscription_id): Option<i64> = values.first().and_then(|v| v.parse().ok()) else {
        return Ok(());
    };

    let Some(subscription) = subscriptions_by_id.get(&subscription_id).cloned() else {
        let embed = error_embed("Error", "The selected subscription no longer exists. Please try again.");
        let response = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new().content("").embed(embed).components(vec![]),
        );
        interaction.create_response(ctx.serenity_context(), response).await?;
        return Ok(());
    };

    // Action the selection
    let host_subscription = subscription.clone();
    let url = host_subscription.get_url();

    ctx.data().subscription_handler.unsubscribe(host_subscription.subscription.clone()).await?;

    let embed = success_embed(
        "Success",
        format!("Unsubscribed from {}. You will no longer be notified when they come online.", url),
    );

    let message = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().embed(embed).content("").components(vec![]),
    );

    interaction.create_response(&ctx.serenity_context(), message).await?;

    Ok(())
}
