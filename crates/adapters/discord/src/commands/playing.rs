use core::time;
use std::collections::HashMap;

use mira_core::{HostSubscription, PersistenceProvider};
use poise::{
    CreateReply,
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

    let user_id = ctx.author().id;

    let host_subscription_by_id: HashMap<i64, HostSubscription> = ctx
        .data()
        .subscription_handler
        .get_subscriptions(guild_id)
        .await?
        .into_iter()
        .filter(|s| s.subscription.is_online()) // filter to only online subscriptions
        .map(|s| (s.subscription.id, s))
        .collect();

    if host_subscription_by_id.is_empty() {
        let embed = error_embed("No Streams", "There are no streams currently online.");
        ctx.send(CreateReply::default().ephemeral(true).embed(embed)).await?;
        return Ok(());
    }

    let options = host_subscription_by_id
        .iter()
        .map(|(id, sub)| CreateSelectMenuOption::new(&sub.get_url(), &id.to_string()))
        .collect();

    let reply = {
        let menu = CreateSelectMenu::new("subscription-select", CreateSelectMenuKind::String { options })
            .placeholder("Choose a stream");
        let components = vec![CreateActionRow::SelectMenu(menu)];
        poise::CreateReply::default().ephemeral(true).content("Pick an option").components(components)
    };

    ctx.send(reply).await?;

    while let Some(interaction) = ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(time::Duration::from_secs(120))
        .filter(move |i| i.user.id == user_id)
        .await
    {
        let selected = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values.get(0),
            _ => None,
        };

        if let Some(value) = selected {
            let subscription_id: i64 = value.parse().expect("Selected subscription_id must be an i64");
            let host_subscription = host_subscription_by_id[&subscription_id].clone();
            let url = host_subscription.get_url();

            ctx.data().subscription_handler.set_playing(subscription_id, playing.clone()).await?;

            let embed = success_embed("Success", format!("Updated {} to playing '{}'", url, playing.clone()));

            let message = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().ephemeral(true).embed(embed).content("").components(vec![]),
            );

            interaction.create_response(&ctx.serenity_context(), message).await?;
        }
    }

    Ok(())
}
