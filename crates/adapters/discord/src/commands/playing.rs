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
pub async fn playing<P: PersistenceProvider>(
    ctx: Context<'_, P>,
    #[description = "What is playing?"] playing: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let embed = error_embed("Playing Failed", "/playing can only be ran from a server channel currently.");
        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    };

    let subscriptions_by_id: HashMap<i64, HostSubscription> = ctx
        .data()
        .subscription_handler
        .get_subscriptions(guild_id)
        .await?
        .into_iter()
        .filter(|s| s.subscription.is_online()) // filter to only online subscriptions
        .map(|s| (s.subscription.id, s))
        .collect();

    if subscriptions_by_id.is_empty() {
        let embed = error_embed("No Streams", "There are no streams currently online.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    }

    let options = subscriptions_by_id
        .iter()
        .map(|(id, sub)| CreateSelectMenuOption::new(&sub.get_url(), &id.to_string()))
        .collect();

    // Create a select menu
    let reply = CreateReply::default().ephemeral(true).content("Choose a stream").components(vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new("subscription-select", CreateSelectMenuKind::String { options })
                .placeholder("Choose a stream"),
        ),
    ]);

    let sent = ctx.send(reply).await?;
    let message_id = sent.message().await?.id;

    // Wait for the selection
    let mut stream = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.message.id == message_id)
        .stream();

    // Handle the timeout
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

    // Action the update
    ctx.data().subscription_handler.set_playing(subscription_id, playing.clone()).await?;

    let embed =
        success_embed("Success", format!("Updated {} to playing '{}'", subscription.get_url(), playing.clone()));

    let message = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().ephemeral(true).embed(embed).content("").components(vec![]),
    );

    interaction.create_response(&ctx.serenity_context(), message).await?;

    Ok(())
}
